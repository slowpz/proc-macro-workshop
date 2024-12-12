use std::ops::RangeInclusive;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{parse::Parse, Error, LitInt, Token};

pub struct Seq {
    range: RangeInclusive<usize>,
}

impl Seq {
    pub fn expand(self) -> TokenStream {
        let impls = self.range.map(|idx| {
            let ident = format_ident!("B{}", idx);
            quote! {
                pub enum #ident {}

                impl Specifier for #ident {
                    const BITS: usize = #idx;
                }

            }
        });

        quote! {
            #(#impls)*
        }
        .into()
    }
}

impl Parse for Seq {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let left: LitInt = input.parse()?;
        let _dot_dot = input.parse::<Token![..]>()?;
        let inclusive = input.parse::<Option<Token![=]>>()?.is_some();
        let right: LitInt = input.parse()?;

        let left = match left.base10_parse::<usize>() {
            Ok(left) => if 0 < left {
                left
            } else {
                return Err(Error::new(Span::call_site(), "rang must be in [1,64]"))
            },
            Err(err) => return Err(err),
        };
        let right = match right.base10_parse::<usize>() {
            Ok(right) => if right <= u64::BITS as usize {
                right
            } else {
                return Err(Error::new(Span::call_site(), "rang must be in [1,64]"))
            }, 
            Err(err) => return Err(err),
        };

        Ok(Self {
            range: if inclusive {
                left..=right
            } else {
                left..=right.saturating_sub(1)
            },
        })
    }
}
