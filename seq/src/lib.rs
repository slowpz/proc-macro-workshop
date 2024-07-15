use std::ops::RangeInclusive;

use proc_macro2::{Group, Punct, Spacing, TokenStream, TokenTree};

use quote::{format_ident, quote};
use syn::{braced, parse::Parse, parse_macro_input, Error, LitInt, Token};

#[proc_macro]
pub fn seq(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as Seq);

    if input.partially_repeat {
        match input.expand(&input.stream, 0) {
            Ok(stream) => stream.into(),
            Err(err) => err.into_compile_error().into(),
        }
    } else {
        let mut res = proc_macro2::TokenStream::new();
        for i in input.range() {
            // 把i和input.ident绑定到一起，替换进input.stream中去
            //添加N次stream到结果中去
            match input.expand(&input.stream, i) {
                Ok(stream) => res.extend(stream),
                Err(err) => return err.into_compile_error().into(),
            }
        }

        res.into()
    }
}

struct Seq {
    ident: syn::Ident,
    range: RangeInclusive<usize>,
    stream: proc_macro2::TokenStream,
    partially_repeat: bool,
}

impl Seq {
    fn expand(&self, token_stream: &TokenStream, i: usize) -> syn::Result<TokenStream> {
        let mut res: Vec<TokenTree> = vec![];

        let tokens = token_stream.clone().into_iter().collect::<Vec<_>>();

        for token_idx in 0..tokens.len() {
            let token = &tokens[token_idx];

            match token {
                TokenTree::Group(group)
                    if is_repeat_group(
                        tokens.get(token_idx.saturating_sub(1)),
                        Some(token),
                        tokens.get(token_idx.saturating_add(1)),
                    )
                    .is_none() =>
                {
                    let stream = self.expand(&group.stream(), i)?;
                    let mut new_group = Group::new(group.delimiter(), stream);
                    //Set the span, so the new stream know which line of code is wrong
                    new_group.set_span(group.span());
                    res.push(TokenTree::Group(new_group));
                }
                TokenTree::Ident(ident) if ident == &self.ident => match res.last() {
                    Some(TokenTree::Punct(punct)) if is_alone_tilde(punct) => {
                        res.pop();
                        let new_ident = if let Some(TokenTree::Ident(prefix)) = res.pop() {
                            let mut new_ident = format_ident!("{}{}", prefix, i);
                            new_ident.set_span(ident.span());
                            new_ident
                        } else {
                            return Err(Error::new_spanned(
                                ident,
                                format!("~{} required a prefix", ident),
                            ));
                        };

                        res.push(TokenTree::Ident(new_ident));
                    }
                    _ => {
                        let mut i = proc_macro2::Literal::usize_unsuffixed(i);
                        i.set_span(ident.span());
                        res.push(TokenTree::Literal(i));
                    }
                },
                TokenTree::Punct(_) => {
                    if let Some(group) = is_repeat_group(
                        tokens.get(token_idx.saturating_sub(2)),
                        tokens.get(token_idx.saturating_sub(1)),
                        Some(token),
                    ) {
                        //pop group and pound
                        res.pop();
                        res.pop();

                        for i in self.range() {
                            let stream = self.expand(&group.stream(), i)?;
                            res.extend(stream);
                        }
                    } else {
                        res.push(token.clone());
                    }
                }
                _ => res.push(token.clone()),
            }
        }

        let res = res.into_iter().fold(TokenStream::new(), |mut acc, e| {
            acc.extend(quote! {#e});
            acc
        });
        Ok(res)
    }

    fn range(&self) -> RangeInclusive<usize> {
        self.range.clone()
    }
}

fn is_alone_tilde(punct: &Punct) -> bool {
    punct.as_char() == '~' && punct.spacing() == Spacing::Alone
}

fn is_repeat_group<'a>(
    pound: Option<&'a TokenTree>,
    group: Option<&'a TokenTree>,
    star: Option<&'a TokenTree>,
) -> Option<&'a Group> {
    match (pound, group, star) {
        (
            Some(TokenTree::Punct(pound)),
            Some(TokenTree::Group(group)),
            Some(TokenTree::Punct(star)),
        ) if pound.as_char() == '#'
            && pound.spacing() == Spacing::Alone
            && star.as_char() == '*'
            && star.spacing() == Spacing::Alone =>
        {
            Some(group)
        }
        _ => None,
    }
}

impl Parse for Seq {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = input.parse()?;
        let _in_token = input.parse::<Token![in]>()?;
        let left: LitInt = input.parse()?;
        let _dot_dot = input.parse::<Token![..]>()?;
        let inclusive = input.parse::<Option<Token![=]>>()?.is_some();
        let right: LitInt = input.parse()?;

        let left = match left.base10_parse::<usize>() {
            Ok(left) => left,
            Err(err) => return Err(err),
        };
        let right = match right.base10_parse::<usize>() {
            Ok(right) => right,
            Err(err) => return Err(err),
        };

        let content;
        let _ = braced!(content in input);
        let stream = content.parse()?;
        let partially_repeat = contain_partially_repeat(&stream);
        Ok(Self {
            ident,
            range: if inclusive {
                left..=right
            } else {
                left..=right.saturating_sub(1)
            },
            stream,
            partially_repeat,
        })
    }
}

fn contain_partially_repeat(token_stream: &TokenStream) -> bool {
    let mut tokens: Vec<TokenTree> = vec![];

    for (idx, token) in token_stream.clone().into_iter().enumerate() {
        match token {
            TokenTree::Group(group) if contain_partially_repeat(&group.stream()) => {
                return true;
            }
            TokenTree::Punct(_)
                if is_repeat_group(
                    tokens.get(idx.saturating_sub(2)),
                    tokens.get(idx.saturating_sub(1)),
                    Some(&token),
                )
                .is_some() =>
            {
                return true;
            }
            _ => tokens.push(token),
        }
    }

    false
}
