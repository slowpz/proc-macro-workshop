use proc_macro2::{Ident, TokenStream};
use quote::quote;
use std::{result::Result, vec};
use syn::{
    parse_macro_input, parse_quote, AngleBracketedGenericArguments, Attribute, Data, DataStruct,
    DeriveInput, Error, Fields, GenericArgument, GenericParam, Generics, Lit, MetaNameValue, Path,
    PathArguments, Type, TypePath, WherePredicate,
};

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let data = parse_macro_input!(input as DeriveInput);

    let stream = match debug_impl(data) {
        Ok(stream) => stream,
        Err(err) => return err.into_compile_error().into(),
    };

    proc_macro::TokenStream::from(stream)
}

//识别对应的Type,是否在PhantomData引用了
//如果是则生成
// impl<T> Debug for Field<T>
//     where
//         PhantomData<T>: Debug,
//     {...}
// 否则
// impl<T: Debug> Debug for Field<T>
//    {...}
//第一步，如何识别T在PhantomData里面使用了？
fn debug_impl(data: DeriveInput) -> Result<TokenStream, Error> {
    let ident = &data.ident;
    let struct_data = if let Data::Struct(st) = &data.data {
        st
    } else {
        return Err(syn::Error::new_spanned(ident, "Expect struct"));
    };
    let fields = fields(struct_data);

    //let phantom_types = type_bounds_handle(&struct_data);
    let generics = add_trait_bounds(struct_data, data.generics, &data.attrs);
    let (impl_generics, ty_genrics, where_clause) = generics.split_for_impl();

    let ident_name = ident.to_string();
    Ok(quote! {
        impl #impl_generics std::fmt::Debug for #ident #ty_genrics #where_clause {
            fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                fmt.debug_struct(#ident_name)
                #(.#fields)*
                .finish()
            }
        }
    })
}

fn scape_hatch(attrs: &[Attribute]) -> Result<Option<String>, Error> {
    let attr = match attrs.last() {
        Some(attr) => attr,
        None => return Ok(None),
    };

    let meta_list = match &attr.meta {
        syn::Meta::List(list) => list,
        _ => return Ok(None),
    };

    match syn::parse2::<MetaNameValue>(meta_list.tokens.clone()) {
        Ok(name_value) => {
            if !name_value.path.is_ident("bound") {
                return Err(syn::Error::new_spanned(
                    name_value.path,
                    "unknow attr name ",
                ));
            }

            let explit = match name_value.value {
                syn::Expr::Lit(lit) => lit,
                _ => return Ok(None),
            };

            match explit.lit {
                Lit::Str(str) => Ok(Some(str.value())),
                _ => Ok(None),
            }
        }
        Err(e) => Err(syn::Error::new_spanned(
            meta_list,
            format!("parse bound attr error:{:?}", e),
        )),
    }
}

fn fields(struct_data: &DataStruct) -> Vec<TokenStream> {
    match struct_data.fields {
        Fields::Named(ref fields) => {
            let mut results = vec![];
            for f in &fields.named {
                let ident = match f.ident.as_ref() {
                    Some(ident) => ident,
                    None => continue,
                };

                let ident_name = ident.to_string();

                let token_stream = match debug_fmt(f) {
                    Ok(Some(str)) => quote!(
                        field(#ident_name, &std::format_args!(#str, &self.#ident))
                    ),
                    Ok(_) => quote!(
                        field(#ident_name, &self.#ident)
                    ),
                    Err(e) => e.into_compile_error(),
                };

                results.push(token_stream);
            }
            results
        }
        _ => vec![],
    }
}

fn type_bounds_handle(struct_data: &DataStruct) -> (Vec<Ident>, Vec<WherePredicate>) {
    let mut handled = vec![];
    let mut types = vec![];
    for field in &struct_data.fields {
        let ty = &field.ty;
        if let Some(ty) = is_type(ty, "PhantomData") {
            match ty {
                Type::Path(type_path) => {
                    if let Some(ident) = type_path.path.get_ident() {
                        handled.push(ident.clone());
                        types.push(parse_quote!(std::marker::PhantomData<#ident>: std::fmt::Debug));
                    }
                }
                Type::Reference(reference) => {
                    if let Type::Path(path) = reference.elem.as_ref() {
                        if let Some(ident) = path.path.get_ident() {
                            handled.push(ident.clone());
                            types.push(
                                parse_quote!(std::marker::PhantomData<#ident>: std::fmt::Debug),
                            );
                        }
                    }
                }
                _ => {}
            };
        } else if let Type::Path(type_path) = ty {
            // handle associated-type
            //Vec<T::Value>: std::fmt::Debug
            //types.push(parse_quote!(#type_path: std::fmt::Debug));
            // Wrong
            //types.push(parse_quote!(#type_path.path: std::fmt::Debug));
            for se in &type_path.path.segments {
                let generics = match &se.arguments {
                    PathArguments::AngleBracketed(generics) => generics,
                    _ => continue,
                };
                for generic in &generics.args {
                    let type_path = match generic {
                        GenericArgument::Type(Type::Path(type_path)) => {
                            // You can identify associated types as any syn::TypePath in which the first
                            // path segment is one of the type parameters and there is more than one
                            // segment.
                            if type_path.path.segments.len() < 2 {
                                continue;
                            } else {
                                type_path
                            }
                        }
                        _ => continue,
                    };

                    match type_path.path.segments.first().map(|se| se.ident.clone()) {
                        Some(ident) => {
                            handled.push(ident);
                            types.push(parse_quote!(#type_path: std::fmt::Debug));
                        }
                        _ => continue,
                    };
                }
            }
        }
    }

    (handled, types)
}

fn debug_fmt(f: &syn::Field) -> syn::Result<Option<String>> {
    let name = None;
    for attr in &f.attrs {
        if !attr.path().is_ident("debug") {
            continue;
        }

        if name.is_some() {
            return Err(Error::new_spanned(&f.ident, "too much builder attrs"));
        }

        return match &attr.meta {
            syn::Meta::NameValue(name_value) => match &name_value.value {
                syn::Expr::Lit(lit) => match &lit.lit {
                    syn::Lit::Str(str) => Ok(Some(str.value())),
                    _ => Err(syn::Error::new_spanned(
                        &attr.meta,
                        "expected `debug = \"...\"`",
                    )),
                },
                _ => Err(syn::Error::new_spanned(
                    attr.meta.path().get_ident(),
                    "Unknow expr",
                )),
            },
            _ => Err(syn::Error::new_spanned(
                attr.meta.path().get_ident(),
                "Unknow meta",
            )),
        };
    }

    Ok(name)
}

// Add a bound `T: Debug` to every type parameter T.

fn add_trait_bounds(
    struct_data: &DataStruct,
    mut generics: Generics,
    attrs: &[Attribute],
) -> Generics {
    if let Ok(Some(lit)) = scape_hatch(attrs) {
        match syn::parse_str(lit.to_string().as_str()) {
            Ok(where_predicated) => {
                let where_clause = generics.make_where_clause();
                where_clause.predicates.push(where_predicated);
            }
            Err(err) => eprintln!("parse where_predicated error:{:?}", err),
        }
    } else {
        let (handled, filed_type_bound) = type_bounds_handle(struct_data);
        for param in &mut generics.params {
            let type_param = if let GenericParam::Type(ref mut type_param) = *param {
                type_param
            } else {
                continue;
            };

            if handled.contains(&type_param.ident) {
                continue;
            }

            type_param.bounds.push(parse_quote!(std::fmt::Debug));
        }

        let where_clause = generics.make_where_clause();
        for ident in filed_type_bound {
            where_clause.predicates.push(ident)
        }
    }

    generics
}

//应该使用全路径的。。。
fn is_type<'a>(ty: &'a Type, ty_str: &str) -> Option<&'a Type> {
    let segments = if let Type::Path(TypePath {
        path: Path { segments, .. },
        ..
    }) = ty
    {
        segments
    } else {
        return None;
    };

    let seg = match segments.last() {
        Some(seg) => seg,
        None => return None,
    };

    if seg.ident != ty_str {
        return None;
    }

    match &seg.arguments {
        PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) => {
            if let Some(GenericArgument::Type(ty)) = args.last() {
                Some(ty)
            } else {
                None
            }
        }
        _ => None,
    }
}
