use proc_macro::TokenStream;
use syn::{braced, parse::Parse, parse_macro_input, Token};

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Seq);

    TokenStream::new()
}

struct Seq {
    ident: syn::Ident,
    in_token: Token![in],
    left: syn::LitInt,
    point_token: Token![..],
    right: syn::LitInt,
    stream: proc_macro2::TokenStream,
}

impl Parse for Seq {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = input.parse()?;
        let in_token = input.parse()?;
        let left = input.parse()?;
        let point_token = input.parse()?;
        let right = input.parse()?;

        let content;
        let _ = braced!(content in input);
        let stream = content.parse()?;
        Ok(Self {
            ident,
            in_token,
            left,
            point_token,
            right,
            stream,
        })
    }
}
