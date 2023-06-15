use proc_macro2::{TokenStream, Ident};
use quote::{quote, format_ident, quote_spanned};
use syn::{parse_macro_input, DeriveInput, Data, Fields, spanned::Spanned};

#[proc_macro_derive(Builder)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    
    let ident = input.ident;

    let builder_name = format_ident!("{}Builder", ident);
    let builder_fields = builder_field(&input.data);
    let builder_setter = builder_setter(&input.data);
    let builder_fn = builder_fn(&ident,&input.data);

    let default_builder_init =  builder_field_default(&input.data);
    let default_builder = quote! {
        #builder_name {
           #default_builder_init
        }
    } ;

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

fn builder_setter(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let recurse = fields.named.iter().map(|f| {
                        if let Some(ident) = f.ident.as_ref() {
                            let name = format_ident!("{}", ident);
                            let file_type = &f.ty;
                            quote_spanned! {f.span()=>
                                pub fn #name(&mut self, val: #file_type) -> &mut Self {
                                    self.#name = Some(val);
                                    self
                                }
                            }
                        } else {
                            quote_spanned!{f.span() => {

                            }}
                        }
                    });
                    quote! {
                        #(#recurse)*
                    }
                },
                _ => unimplemented!(),
            }
        }
        _ => unimplemented!(),
    }
}


fn builder_fn(ident: &Ident, data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let field_extract = fields.named.iter().map(|f| {
                        if let Some(name) = f.ident.as_ref() {
                            let msg = format!("missing {}", name);
                            quote_spanned! {f.span()=>
                                let #name = self.#name.clone().ok_or_else(|| #msg )?;
                            }
                        } else {
                            quote_spanned!{f.span() => {

                            }}
                        }
                    });
                    let field_init = fields.named.iter().map(|f| {
                        if let Some(ident) = f.ident.as_ref() {
                            quote_spanned! {f.span()=>
                                #ident,
                            }
                        } else {
                            quote_spanned!{f.span() => {

                            }}
                        }
                    });
                    quote! {
                        pub fn build(&mut self) -> Result<#ident, Box<dyn Error>> {
                            use std::error::Error;
                            #(#field_extract)*

                            Ok(#ident {
                                #(#field_init)*
                            })
                        }
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