use proc_macro2::TokenStream;
use quote::{quote};
use syn::{parse_macro_input, Data, DeriveInput, Error, Fields};

#[proc_macro_derive(CustomDebug)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let data = parse_macro_input!(input as DeriveInput);

    let stream = match debug_impl(&data) {
        Ok(stream) => stream,
        Err(err) => return err.into_compile_error().into(),
    };

    proc_macro::TokenStream::from(stream)
}

fn debug_impl(data: &DeriveInput) -> Result<TokenStream, Error> {
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

                quote!(
                    field(#ident_name, &self.#ident)
                )
            });

            recurse
        }
        _ => return Err(Error::new_spanned(ident, "only support named field")),
    };

    let ident_name = ident.to_string();
    Ok(quote! {
        impl std::fmt::Debug for #ident {
            fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                fmt.debug_struct(#ident_name)
                #(.#fields)*
                .finish()
            }
        }
    })
}
