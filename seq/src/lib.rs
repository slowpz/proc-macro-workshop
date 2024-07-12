use proc_macro2::{Group, TokenStream, TokenTree};

use quote::quote;
use syn::{braced, parse::Parse, parse_macro_input, Token};

#[proc_macro]
pub fn seq(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as Seq);

    let left = match input.left.base10_parse::<usize>() {
        Ok(left) => left,
        Err(err) => return err.into_compile_error().into(),
    };
    let right = match input.right.base10_parse::<usize>() {
        Ok(right) => right,
        Err(err) => return err.into_compile_error().into(),
    };

    let mut res = proc_macro2::TokenStream::new();
    for i in left..right {
        // 把i和input.ident绑定到一起，替换进input.stream中去
        //添加N次stream到结果中去
        match input.expand(&input.stream, i) {
            Ok(stream) => res.extend(stream),
            Err(err) => return err.into_compile_error().into(),
        }
    }

    res.into()
}

struct Seq {
    ident: syn::Ident,
    left: syn::LitInt,
    right: syn::LitInt,
    stream: proc_macro2::TokenStream,
}

impl Seq {
    fn expand(&self, token_stream: &TokenStream, i: usize) -> syn::Result<TokenStream> {
        let mut res = proc_macro2::TokenStream::new();
        let tokens = token_stream.clone().into_iter().collect::<Vec<_>>();

        for token in tokens {
            match token {
                TokenTree::Group(group) => {
                    let stream = self.expand(&group.stream(), i)?;
                    let mut new_group = Group::new(group.delimiter(), stream);
                    new_group.set_span(group.span());
                    res.extend(quote! {#new_group});
                }
                TokenTree::Ident(ident) if ident == self.ident => {
                    let i = proc_macro2::Literal::usize_unsuffixed(i);
                    res.extend(quote! {#i});
                }
                _ => res.extend(quote! {#token}),
            }
        }
        Ok(res)
    }
}

impl Parse for Seq {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = input.parse()?;
        let _in_token = input.parse::<Token![in]>()?;
        let left = input.parse()?;
        let _dot_dot = input.parse::<Token![..]>()?;
        let right = input.parse()?;

        let content;
        let _ = braced!(content in input);
        let stream = content.parse()?;
        Ok(Self {
            ident,
            left,
            right,
            stream,
        })
    }
}
