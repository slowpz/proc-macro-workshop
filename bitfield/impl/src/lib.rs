mod gen;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote, ToTokens};
use syn::{parse_macro_input, spanned::Spanned, Item};

#[proc_macro_attribute]
pub fn bitfield(_: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as syn::Item);
    match struct_bitfield(&item) {
        Ok(item) => item,
        Err(err) => {
            let mut err = err.to_compile_error();
            err.extend(item.to_token_stream());
            err.into()
        }
    }
}

fn struct_bitfield(item: &syn::Item) -> syn::Result<TokenStream> {
    match item {
        Item::Struct(item) => {
            let vis = &item.vis;
            let ident = &item.ident;
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

            let (getters, setters) = getters_and_setters(item);
            let size_check = check_size_is_multiple_of_eights(item);

            Ok(quote! {

                #size_check

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

fn check_size_is_multiple_of_eights(item: &syn::ItemStruct) -> proc_macro2::TokenStream {
    let size = item.fields.iter().map(|f| {
        let ty = &f.ty;
        quote! { <#ty as bitfield::Specifier>::BITS }
    });

    quote! {
        const _: () = {
            const SIZE: usize = #(#size)+*;

            if SIZE % 8 != 0 {
                panic!("size must be multiple of eight bytes")
            }
        };
    }
}

fn getters_and_setters(
    item: &syn::ItemStruct,
) -> (Vec<proc_macro2::TokenStream>, Vec<proc_macro2::TokenStream>) {
    let getters = {
        let mut getters = vec![];
        let mut offsets = vec![quote! { 0 }];
        for f in &item.fields {
            match f.ident.as_ref() {
                Some(ident) => {
                    let ty = &f.ty;
                    let ty = quote! { <#ty as bitfield::Specifier> };
                    let width = quote! { #ty::BITS };
                    let assosiate_ty = quote! { #ty::T };
                    let fn_ident = format_ident!("get_{}", ident);

                    getters.push(quote! {
                        pub fn #fn_ident(&self) -> #assosiate_ty {
                            const BIT_OFFSET_START: usize = #(#offsets)+*;
                            #ty::get(&self.data, BIT_OFFSET_START)
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
                    let ty = quote! { <#ty as bitfield::Specifier> };
                    let width = quote! { #ty::BITS };
                    let assosiate_ty = quote! { #ty::T };
                    let fn_ident = format_ident!("set_{}", ident);

                    setters.push(quote! {
                        pub fn #fn_ident(&mut self, val: #assosiate_ty) {
                            const BIT_OFFSET_START: usize = #(#offsets)+*;
                            #ty::set(&mut self.data, BIT_OFFSET_START, val);
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
    (getters, setters)
}

#[proc_macro]
pub fn specifiers(input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as gen::Seq);
    item.expand()
}

#[proc_macro_derive(BitfieldSpecifier)]
pub fn bitfield_specifier(input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as syn::Item);
    match enum_specifier(&item) {
        Ok(item) => item.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn enum_specifier(item: &syn::Item) -> syn::Result<proc_macro2::TokenStream> {
    match item {
        Item::Enum(syn::ItemEnum {
            ident, variants, ..
        }) => {
            let mut bits = 0u32;
            let mut to_lit_match = vec![];
            let mut to_enum_match = vec![];
            for var in variants {
                let lit = match &var.discriminant {
                    Some((_, syn::Expr::Lit(syn::ExprLit { lit, .. }))) => lit,
                    _ => {
                        return Err(syn::Error::new(
                            var.span(),
                            "expected explicit discriminant value",
                        ))
                    }
                };
                bits = match lit {
                    syn::Lit::Int(lit_int) => {
                        let leading_zeros = lit_int.base10_parse::<u128>()?.leading_zeros();
                        bits.max(u128::BITS - leading_zeros)
                    }
                    _ => {
                        return Err(syn::Error::new(
                            lit.span(),
                            "expected explicit discriminant value",
                        ))
                    }
                };

                let var_ident = &var.ident;
                to_lit_match.push(quote! {
                   Self::T::#var_ident => #lit
                });

                to_enum_match.push(quote! {
                    #lit => Self::T::#var_ident
                });
            }

            let bits = bits as usize;
            
            let to_lit_match = quote! {
                match val {
                    #(#to_lit_match),*
                };
            };

            let to_enum_match = {
                quote! {
                    match lit {
                        #(#to_enum_match),*,
                        _ => panic!("Invalid value {}", lit)
                    }
                }
            };

            let val_getter = {
                let ident = format_ident!("B{}", bits);
                quote! { <#ident as bitfield::Specifier>::get(data, bit_offset) }
            };

            let val_setter = {
                let ident = format_ident!("B{}", bits);
                quote! { <#ident as bitfield::Specifier>::set(data, bit_offset, lit) }
            };

            Ok(quote! {
                impl bitfield::Specifier for #ident {
                    const BITS: usize = #bits;

                    type T = #ident;

                    fn set(data:&mut [u8], bit_offset: usize, val: Self::T) {
                        let lit = #to_lit_match;
                        #val_setter;
                    }

                    fn get(data:&[u8], bit_offset: usize) -> Self::T {
                        let lit = #val_getter;
                        #to_enum_match
                    }
                }
            })
        }
        _ => Err(syn::Error::new(Span::call_site(), "expected Enum")),
    }
}