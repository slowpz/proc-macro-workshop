use proc_macro2::TokenStream;
use quote::quote;
use std::result::Result;
use syn::{parse_macro_input, Data, DeriveInput, Error, Fields, Generics, GenericParam, parse_quote};

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let data = parse_macro_input!(input as DeriveInput);

    let stream = match debug_impl(data) {
        Ok(stream) => stream,
        Err(err) => return err.into_compile_error().into(),
    };

    proc_macro::TokenStream::from(stream)
}

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

    let generics = add_trait_bounds(data.generics);
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
fn add_trait_bounds(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(std::fmt::Debug));
        }
    }
    generics
}