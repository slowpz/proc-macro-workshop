use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::{
    parse_macro_input, punctuated::Punctuated, spanned::Spanned, AngleBracketedGenericArguments,
    Data, DeriveInput, Error, Expr, ExprLit, Fields, GenericArgument, Lit, Meta, Path,
    PathArguments, Result, Token, Type, TypePath,
};

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;

    let builder_fields = match builder_field(&input.data, &ident) {
        Ok(field) => field,
        Err(err) => return err.into_compile_error().into(),
    };

    let builder_setter = match builder_setter(&input.data, &ident) {
        Ok(setter) => setter,
        Err(err) => return err.into_compile_error().into(),
    };

    let builder_fn = match builder_fn(&input.data, &ident) {
        Ok(function) => function,
        Err(err) => return err.into_compile_error().into(),
    };

    let default_builder_init = match builder_field_default(&input.data, &ident) {
        Ok(default_fields) => default_fields,
        Err(err) => return err.into_compile_error().into(),
    };

    let builder_name = format_ident!("{}Builder", ident);
    let default_builder = quote! {
        #builder_name {
           #default_builder_init
        }
    };

    let builder = quote! {
        use std::error::Error;
        impl #ident {
            fn builder() -> #builder_name {
                #default_builder
            }
        }

        struct #builder_name {
          #builder_fields
        }

        impl #builder_name {
            #builder_setter

            #builder_fn
        }
    };

    proc_macro::TokenStream::from(builder)
}

fn builder_field(data: &Data, ident: &Ident) -> Result<TokenStream> {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let recurse = fields.named.iter().map(|f| {
                    let ident = match f.ident.as_ref() {
                        Some(ident) => ident,
                        None => {
                            return Error::new_spanned(ident, "anyonymous filed is not support")
                                .into_compile_error()
                        }
                    };

                    if let Some(ty) = is_type(&f.ty, "Vec") {
                        quote_spanned! {f.span()=>
                            #ident : Vec<#ty>
                        }
                    } else {
                        let ty = match is_type(&f.ty, "Option") {
                            Some(ty) => ty,
                            None => &f.ty,
                        };
                        quote_spanned! {f.span()=>
                            #ident : Option<#ty>
                        }
                    }
                });

                Ok(quote! {
                    #(#recurse),*
                })
            }
            _ => Err(Error::new_spanned(ident, "only support named field")),
        },
        _ => Err(Error::new_spanned(ident, "only support struct")),
    }
}
fn builder_setter(data: &Data, ident: &Ident) -> Result<TokenStream> {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let recurse = fields.named.iter().map(|f| {
                    let ident = match f.ident.as_ref() {
                        Some(ident) => ident,
                        None => return Error::new_spanned(&f.ident, "anyonymous filed is not support")
                                .into_compile_error()
                    };
                    if let Some(vec_component_ty) = is_type(&f.ty, "Vec") {
                        match builder_each_attr(f) {
                            Ok(Some(builder_name)) => {
                                if ident == &builder_name {
                                    quote_spanned! {f.span()=>
                                        pub fn #ident(&mut self, val: #vec_component_ty) -> &mut Self {
                                            self.#ident.push(val);
                                            self
                                        }
                                    }
                                } else {
                                    let builder_ident = format_ident!("{}", builder_name);
                                    quote_spanned! {f.span()=>
                                        pub fn #ident(&mut self, val: Vec<#vec_component_ty>) -> &mut Self {
                                            self.#ident = val;
                                            self
                                        }

                                        pub fn #builder_ident(&mut self, val: #vec_component_ty) -> &mut Self {
                                            self.#ident.push(val);
                                            self
                                        }
                                    }
                                }
                            },
                            Ok(None) => quote_spanned! {f.span()=>
                                pub fn #ident(&mut self, val: Vec<#vec_component_ty>) -> &mut Self {
                                    self.#ident = val;
                                    self
                                }
                            },
                            Err(e) => Error::into_compile_error(e),
                        }
                    } else {
                        let file_type = match is_type(&f.ty, "Option") {
                            Some(ty) => ty,
                            None => &f.ty,
                        };
                        quote_spanned! {f.span()=>
                            pub fn #ident(&mut self, val: #file_type) -> &mut Self {
                                self.#ident = Some(val);
                                self
                            }
                        }
                    }
                });
                Ok(quote! {
                    #(#recurse)*
                })
            }
            _ => Err(Error::new_spanned(ident, "only named filed support")),
        },
        _ => Err(Error::new_spanned(ident, "onply support struct")),
    }
}

fn builder_each_attr(f: &syn::Field) -> Result<Option<String>> {
    match is_type(&f.ty, "Vec") {
        Some(_) => {}
        None => return Ok(None),
    };

    let mut name = None;
    for attr in &f.attrs {
        if !attr.path().is_ident("builder") {
            continue;
        }

        if name.is_some() {
            return Err(Error::new_spanned(&f.ident, "too much builder attrs"));
        }

        let nested = attr.parse_args_with(Punctuated::<Meta, Token![=]>::parse_terminated);

        match nested {
            Ok(nested) => {
                for meta in &nested {
                    match meta {
                        Meta::NameValue(name_value) => {
                            if !name_value.path.is_ident("each") {
                                return Err(syn::Error::new_spanned(
                                    &attr.meta,
                                    "expected `builder(each = \"...\")`",
                                ));
                            }

                            match &name_value.value {
                                Expr::Lit(ExprLit { lit, .. }) => {
                                    if let Lit::Str(str) = lit {
                                        name = Some(str.value());
                                    } else {
                                        return Err(syn::Error::new_spanned(
                                            &attr.meta,
                                            "expected `builder(each = \"...\")`",
                                        ));
                                    }
                                }
                                _ => {
                                    return Err(syn::Error::new_spanned(
                                        &attr.meta,
                                        "expected `builder(each = \"...\")`",
                                    ))
                                }
                            }
                        }
                        _ => {
                            return Err(syn::Error::new_spanned(
                                meta.path().get_ident(),
                                "Unknow meta",
                            ));
                        }
                    }
                }
            }
            Err(err) => {
                return Err(Error::new_spanned(
                    &f.ident,
                    format!("get filed attrs error:{}", err),
                ));
            }
        }
    }

    Ok(name)
}

fn builder_fn(data: &Data, ident: &Ident) -> Result<TokenStream> {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let field_extract = fields.named.iter().map(|f| {
                    let ident = match f.ident.as_ref() {
                        Some(ident) => ident,
                        None => {
                            return Error::new_spanned(ident, "anyonymous filed is not support")
                                .into_compile_error()
                        }
                    };

                    if is_type(&f.ty, "Option").is_some() || is_type(&f.ty, "Vec").is_some() {
                        quote_spanned! {f.span()=>
                             #ident: self.#ident.clone()
                        }
                    } else {
                        let msg = format!("missing {}", ident);
                        quote_spanned! {f.span()=>
                             #ident: self.#ident.clone().ok_or_else(|| #msg )?
                        }
                    }
                });
                Ok(quote! {
                    pub fn build(&mut self) -> Result<#ident, Box<dyn Error>> {
                        use std::error::Error;
                        Ok(#ident {
                            #(#field_extract,)*
                        })
                    }
                })
            }
            _ => Err(Error::new_spanned(ident, "anyonymous filed is not support")),
        },
        _ => Err(Error::new_spanned(ident, "only struct support")),
    }
}

fn builder_field_default(data: &Data, ident: &Ident) -> Result<TokenStream> {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let recurse = fields.named.iter().map(|f| {
                    let ident = match f.ident.as_ref() {
                        Some(ident) => ident,
                        None => {
                            return Error::new_spanned(ident, "anyonymous filed is not support")
                                .into_compile_error()
                        }
                    };

                    if is_type(&f.ty, "Vec").is_some() {
                        quote_spanned! {f.span()=>
                            #ident : Vec::new()
                        }
                    } else {
                        quote_spanned! {f.span()=>
                            #ident : None
                        }
                    }
                });
                Ok(quote! {
                    #(#recurse),*
                })
            }
            _ => Err(Error::new_spanned(ident, "anyonymous filed is not support")),
        },
        _ => Err(Error::new_spanned(ident, "only struct support")),
    }
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
            if let Some(GenericArgument::Type(ty)) = args.first() {
                Some(ty)
            } else {
                None
            }
        }
        _ => None,
    }
}
