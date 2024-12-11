use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Item};

#[proc_macro_attribute]
pub fn bitfield(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let item = parse_macro_input!(input as syn::Item);
    match struct_bitefield(item) {
        Ok(item) => item,
        Err(err) => {
            let err = err.to_compile_error();
            err.into()
        }
    }
}

fn struct_bitefield(item: syn::Item) -> syn::Result<TokenStream> {
    match item {
        Item::Struct(item) => {
            let vis = item.vis;
            let ident = item.ident;
            let (_, ty_genrics, where_clause) = item.generics.split_for_impl();
            let lengths = {
                let tys = item.fields.iter().map(|f| {
                    let ty = &f.ty;
                    quote! { <#ty as bitfield::Specifier>::BITS }
                });

                quote! {
                    (#(#tys)+*) / u8::BITS as usize
                }
            };

            let getters = item.fields.iter().map(|f| {
                let ident = match f.ident.as_ref() {
                    Some(ident) => ident,
                    None => {
                        return syn::Error::new_spanned(f, "anyonymous filed is not support")
                            .into_compile_error()
                    }
                };

                let fn_ident = format_ident!("get_{}", ident);
                quote! {
                    pub fn #fn_ident(&self) -> u64 {
                        unimplemented!();
                    }
                }
            });

            
            let setters = item.fields.iter().map(|f| {
                let ident = match f.ident.as_ref() {
                    Some(ident) => ident,
                    None => {
                        return syn::Error::new_spanned(f, "anyonymous filed is not support")
                            .into_compile_error()
                    }
                };

                let fn_ident = format_ident!("set_{}", ident);
                quote! {
                    pub fn #fn_ident(&mut self, v: u64) {
                        unimplemented!();
                    }
                }
            });


            Ok(quote! {

                #[repr(C)]
                #vis struct #ident #ty_genrics #where_clause {
                    data:[u8; #lengths]
                }

                impl #ident {

                    pub fn new() -> Self {
                        Self {
                            data:[0;#lengths]
                        }
                    }

                    #(#getters)*

                    #(#setters)*
                }

            }
            .into())
        }
        _ => Err(syn::Error::new(
            Span::call_site(),
            "expected enum or match expression",
        )),
    }
}
