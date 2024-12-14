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
            let bites = match idx {
                1..=8 => format_ident!("u8"),
                9..=16 => format_ident!("u16"),
                17..=32 => format_ident!("u32"),
                33..=64 => format_ident!("u64"),
                65..=128 => format_ident!("u128"),
                _ => {
                    return syn::Error::new(
                        Span::call_site(),
                        format!("size must be in range of [1..64]. current size:{}", idx),
                    )
                    .to_compile_error();
                }
            };
            quote! {
                pub enum #ident {}

                impl Specifier for #ident {
                    const BITS: usize = #idx;
                    type T = #bites;

                    fn get(data:&[u8], bit_offset: usize) -> Self::T {
                        let bit_offset_end = bit_offset + Self::BITS;
                        let mut val = 0;
                        for (shift, bit_idx) in (bit_offset..bit_offset_end).enumerate() {
                            // 方法：n>>k   等价于  n/(2^k)
                            let idx = bit_idx >> 3 as usize;
                            if (data[idx] & 1u8.rotate_left(bit_idx as u32)) != 0 {
                                val |= 1 << shift;
                            }
                        }
                        val
                    }

                    fn set(data:&mut [u8], bit_offset:usize, val: Self::T) {
                        let bit_offset_end = bit_offset + Self::BITS;
                        let mut val = val;
                        for bit_idx in bit_offset..bit_offset_end {
                            // 方法：n>>k   等价于  n/(2^k)
                            let idx = bit_idx >> 3 as usize;
                            if val & 0b1 == 1 {
                                data[idx] |= 1u8.rotate_left(bit_idx as u32);
                            } else {
                                data[idx] &= !(1u8.rotate_left(bit_idx as u32));
                            }
                            val = val.rotate_right(1);
                        }
                    }
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
            Ok(left) => {
                if 0 < left {
                    left
                } else {
                    return Err(Error::new(Span::call_site(), "rang must be in [1,64]"));
                }
            }
            Err(err) => return Err(err),
        };
        let right = match right.base10_parse::<usize>() {
            Ok(right) => {
                if right <= u128::BITS as usize {
                    right
                } else {
                    return Err(Error::new(Span::call_site(), "rang must be in [1,64]"));
                }
            }
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
