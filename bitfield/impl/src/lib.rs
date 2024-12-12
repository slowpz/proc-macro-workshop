mod gen;

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

            let getters = {
                let mut getters = vec![];
                let mut offsets = vec![quote! { 0 }];
                for f in &item.fields {
                    match f.ident.as_ref() {
                        Some(ident) => {
                            let ty = &f.ty;
                            let width = quote! { <#ty as bitfield::Specifier>::BITS };
                            let fn_ident = format_ident!("get_{}", ident);

                            getters.push(quote! {
                                pub fn #fn_ident(&self) -> u64 {
                                    const BIT_OFFSET_START: usize = #(#offsets)+*;
                                    const BIT_OFFSET_END: usize = BIT_OFFSET_START + #width;
                                    let mut val = 0u64;
                                    for (shift,bit_idx) in (BIT_OFFSET_START..BIT_OFFSET_END).enumerate() {
                                        // 方法：n>>k   等价于  n/(2^k)
                                        let idx = bit_idx >> 3 as usize;
                                        if (self.data[idx] & 1u8.rotate_left(bit_idx as u32)) != 0 {
                                            val |= 1 << shift;
                                        }
                                    }
                                    val
                                }
                            });

                            offsets.push(width);
                        }
                        None => getters.push(
                            syn::Error::new_spanned(f, "anyonymous filed is not support")
                                .into_compile_error(),
                        ),
                    }
                }

                getters
            };

            let setters = {
                let mut setters = vec![];
                let mut offsets = vec![quote! { 0 }];
                for f in &item.fields {
                    match f.ident.as_ref() {
                        Some(ident) => {
                            let ty = &f.ty;
                            let width = quote! { <#ty as bitfield::Specifier>::BITS };
                            let fn_ident = format_ident!("set_{}", ident);

                            setters.push(quote! {
                                pub fn #fn_ident(&mut self, val: u64) {
                                    const BIT_OFFSET_START: usize = #(#offsets)+*;
                                    const BIT_OFFSET_END: usize = BIT_OFFSET_START + #width;
                                    let mut val = val;
                                    for bit_idx in BIT_OFFSET_START..BIT_OFFSET_END {
                                        // 方法：n>>k   等价于  n/(2^k)
                                        let idx = bit_idx >> 3 as usize;
                                        if val & 0b1 == 1 {
                                            self.data[idx] |= 1u8.rotate_left(bit_idx as u32);
                                         } else {
                                            self.data[idx] &= !(1u8.rotate_left(bit_idx as u32));
                                        }
                                        val = val.rotate_right(1);
                                    }
                                }
                            });

                            offsets.push(width);
                        }
                        None => setters.push(
                            syn::Error::new_spanned(f, "anyonymous filed is not support")
                                .into_compile_error(),
                        ),
                    }
                }

                setters
            };

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
        _ => Err(syn::Error::new(Span::call_site(), "expected struct")),
    }
}

#[proc_macro]
pub fn specifiers(input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as gen::Seq);
    item.expand()
}
