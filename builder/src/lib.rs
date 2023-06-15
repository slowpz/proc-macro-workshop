use proc_macro2::TokenStream;
use quote::{quote, format_ident, quote_spanned};
use syn::{parse_macro_input, DeriveInput, Data, Fields, spanned::Spanned};

#[proc_macro_derive(Builder)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    
    let ident = input.ident;

    let builder_name = format_ident!("{}Builder", ident);
    let builder_fields = builder_field(&input.data);
    let builder_field_default = builder_field_default(&input.data);

    let builder = quote! {
        impl #ident {
            fn builder() -> #builder_name {
                #builder_name {
                    #builder_field_default
                }
            }
        }

        struct #builder_name {
          #builder_fields
        }

        impl #builder_name {

        }
    };

    proc_macro::TokenStream::from(builder)
}


fn builder_field(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let recurse = fields.named.iter().map(|f| {
                        if let Some(ident) = f.ident.as_ref() {
                            let name = format_ident!("{}", ident);
                            let file_type = &f.ty;
                            quote_spanned! {f.span()=>
                                #name : Option<#file_type>
                            }
                        } else {
                            quote_spanned!{f.span() => {

                            }}
                        }
                    });
                    quote! {
                        #(#recurse),*
                    }
                },
                _ => unimplemented!(),
            }
        }
        _ => unimplemented!(),
    }
}

fn builder_field_default(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let recurse = fields.named.iter().map(|f| {
                        if let Some(ident) = f.ident.as_ref() {
                            let name = format_ident!("{}", ident);
                            quote_spanned! {f.span()=>
                                #name : None
                            }
                        } else {
                            quote_spanned!{f.span() => {

                            }}
                        }
                    });
                    quote! {
                        #(#recurse),*
                    }
                },
                _ => unimplemented!(),
            }
        }
        _ => unimplemented!(),
    }
}