use proc_macro2::{Ident, TokenStream};
use quote::quote;
use std::{result::Result, vec};
use syn::{
    parse_macro_input, parse_quote, AngleBracketedGenericArguments, Data, DeriveInput, Error,
    Fields, GenericArgument, GenericParam, Generics, Path, PathArguments, Type, TypePath,
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

    let fields = match struct_data.fields {
        Fields::Named(ref fields) => {
            let recurse = fields.named.iter().map(|f| {
                let ident = match f.ident.as_ref() {
                    Some(ident) => ident,
                    None => {
                        return Error::new_spanned(ident, "anyonymous filed is not support")
                            .into_compile_error()
                    }
                };

                let ident_name = ident.to_string();

                match debug_fmt(f) {
                    Ok(Some(str)) => quote!(
                        field(#ident_name, &std::format_args!(#str, &self.#ident))
                    ),
                    Ok(_) => quote!(
                        field(#ident_name, &self.#ident)
                    ),
                    Err(e) => e.into_compile_error(),
                }
            });

            recurse
        }
        _ => return Err(Error::new_spanned(ident, "only support named field")),
    };

    let phantom_types = phantom_types(&data);
    let generics = add_trait_bounds(data.generics, phantom_types);
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

fn phantom_types(data: &DeriveInput) -> Vec<Ident> {
    let struct_data = if let Data::Struct(st) = &data.data {
        st
    } else {
        return vec![];
    };

    let mut types = vec![];
    for field in &struct_data.fields {
        let ty = &field.ty;
        if let Some(ty) = is_type(ty, "PhantomData") {
            match ty {
                Type::Path(path) => {
                    if let Some(ident) = path.path.get_ident() {
                        types.push(ident.clone());
                    }
                }
                Type::Reference(reference) => {
                    if let Type::Path(path) = reference.elem.as_ref() {
                        if let Some(ident) = path.path.get_ident() {
                            types.push(ident.clone());
                        }
                    }
                }
                _ => {}
            };
        }
    }

    types
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

// Add a bound `T: HeapSize` to every type parameter T.
fn add_trait_bounds(mut generics: Generics, phantom_types: Vec<Ident>) -> Generics {
    let mut phantoms = vec![];
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            let ident = &type_param.ident;
            if phantom_types.contains(ident) {
                phantoms.push(parse_quote!(std::marker::PhantomData<#ident>: std::fmt::Debug));
            } else {
                type_param.bounds.push(parse_quote!(std::fmt::Debug));
            }
        }
    }
    let where_clause = generics.make_where_clause();
    for ident in phantoms {
        where_clause.predicates.push(ident)
    }
    generics
}

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
