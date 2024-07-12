use proc_macro::TokenStream;
use syn::{braced, parse::Parse, parse_macro_input, DeriveInput, Token};

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
}

impl Parse for Seq {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let seq = Self {
            ident: input.parse()?,
            in_token: input.parse()?,
            left: input.parse()?,
            point_token: input.parse()?,
            right: input.parse()?,
        };
        let _content;
        let _brace_token = braced!(_content in input);
        Ok(seq)
    }
}
