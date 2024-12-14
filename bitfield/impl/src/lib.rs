mod gen;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote, ToTokens};
use syn::{parse_macro_input, Item};

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
        Item::Enum(en) => {
            let ident = &en.ident;
            let variants = &en.variants;
            if variants.len() % 2 != 0 {
                return Err(syn::Error::new(
                    Span::call_site(),
                    "BitfieldSpecifier expected a number of variants which is a power of 2",
                ));
            }

            let bits = {
                let len = variants.len() as f32;
                f32::ceil(len.log2()) as usize
            };
            let bit_ident = {
                let ident = format_ident!("B{}", bits);
                quote! { <#ident as bitfield::Specifier>}
            };
            let val_getter = quote! { #bit_ident::get(data, bit_offset) };
            let val_setter = quote! { #bit_ident::set(data, bit_offset, lit) };
            let enums = variants.iter().map(|v| {
                let ident = &v.ident;
                quote! { Self::T::#ident }
            });

            let enum_u8s = {
                let enums = enums.clone();
                quote! { [#(#enums as u8),*] }
            };
            let check_discriminant = check_discriminant(en);

            Ok(quote! {

                #check_discriminant

                impl bitfield::Specifier for #ident {
                    const BITS: usize = #bits;

                    type T = #ident;

                    fn set(data:&mut [u8], bit_offset: usize, val: Self::T) {
                        let lit = val as #bit_ident::T;
                        #val_setter;
                    }

                    fn get(data:&[u8], bit_offset: usize) -> Self::T {
                        let lit = #val_getter;
                        #enum_u8s
                        .into_iter()
                        .zip([#(#enums),*])
                        .find(|(val, _)| {
                            val == &lit
                        }).unwrap().1
                    }
                }
            })
        }
        _ => Err(syn::Error::new(Span::call_site(), "expected Enum")),
    }
}


fn check_discriminant(en : &syn::ItemEnum) -> proc_macro2::TokenStream {
    let bits = usize::BITS - (en.variants.len() - 1).leading_zeros();
    let discriminant_type = {
        let ty = format_ident!("B{}", bits);
        quote! { <#ty as bitfield::Specifier>::T }
    };
    let enum_ident = &en.ident;
    let discriminants = en.variants.iter().enumerate().map(|(idx, e)| {
        let ident = format_ident!("a{}", idx);
        let var_ident = &e.ident;
        let str = format!("{}::{} discriminant value out of range, expect range [0,{})", enum_ident, var_ident, en.variants.len());
        quote! {  
            let #ident = TYPE_BITS - (#enum_ident::#var_ident as #discriminant_type).leading_zeros();
            if EXPECT_BITS < #ident {
                panic!(#str);
            }
        }
    });

 

    quote! {
        const _: () =  {
            const TYPE_BITS: u32 = #discriminant_type::BITS;
            const EXPECT_BITS: u32 = #bits;

            #(#discriminants)*
        };
    }
}