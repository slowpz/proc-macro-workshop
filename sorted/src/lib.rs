use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::visit_mut::VisitMut;
use syn::{parse_macro_input, spanned::Spanned, Item};
use syn::{ExprMatch, Pat};

#[proc_macro_attribute]
pub fn sorted(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as syn::Item);
    match enum_sorted(&item) {
        Ok(()) => item.to_token_stream().into(),
        Err(err) => {
            let mut err = err.to_compile_error();
            err.extend(item.to_token_stream());
            err.into()
        }
    }
}

fn enum_sorted(item: &syn::Item) -> syn::Result<()> {
    let item = if let Item::Enum(item_enum) = item {
        item_enum
    } else {
        return Err(syn::Error::new(
            Span::call_site(),
            "expected enum or match expression",
        ));
    };

    let idents = {
        let mut idents = item.variants.iter().collect::<Vec<_>>();
        idents.sort_by_key(|var| &var.ident);
        idents
    };
    for (sorted, origin) in idents.iter().zip(item.variants.iter()) {
        if sorted.ident != origin.ident {
            return Err(syn::Error::new(
                sorted.span(),
                format!("{} should sort before {}", sorted.ident, origin.ident),
            ));
        }
    }

    Ok(())
}

#[proc_macro_attribute]
pub fn check(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(input as syn::Item);
    match match_sorted(&mut item) {
        Ok(()) => item.to_token_stream().into(),
        Err(err) => {
            let mut err = err.into_compile_error();
            err.extend(item.to_token_stream());
            err.into()
        }
    }
    // 都提示你了还不看文档，浪费时间
    // let sorted = item.block.stmts.iter().any(|stmt| {
    //     if let syn::Stmt::Expr(Expr::Match(expr_match), _) = stmt {
    //         expr_match
    //             .attrs
    //             .iter()
    //             .any(|attr| attr.path().is_ident("sorted"))
    //     } else {
    //         false
    //     }
    // });

    // if !sorted {
    //     return input;
    // }

    // let mut stmts = Vec::with_capacity(item.block.stmts.len());

    // for stmt in item.block.stmts.into_iter() {
    //     let (expr_match, semicolon) = match stmt {
    //         Stmt::Expr(Expr::Match(expr_match), semicolon)
    //             if expr_match
    //                 .attrs
    //                 .iter()
    //                 .any(|attr| attr.path().is_ident("sorted")) =>
    //         {
    //             (expr_match, semicolon)
    //         }
    //         _ => {
    //             stmts.push(stmt);
    //             continue;
    //         }
    //     };

    //     let attrs = expr_match
    //         .attrs
    //         .into_iter()
    //         .filter(|attr| !attr.path().is_ident("sorted"))
    //         .collect();
    //     let new_match = ExprMatch {
    //         attrs,
    //         match_token: expr_match.match_token,
    //         expr: expr_match.expr,
    //         brace_token: expr_match.brace_token,
    //         arms: expr_match.arms,
    //     };

    //     stmts.push(Stmt::Expr(Expr::Match(new_match), semicolon));
    // }

    // let item = Item::Fn(ItemFn {
    //     attrs: item.attrs,
    //     vis: item.vis,
    //     sig: item.sig,
    //     block: Block {
    //         brace_token: item.block.brace_token,
    //         stmts,
    //     }
    //     .into(),
    // });

    // TokenStream::from(item.into_token_stream())
}

fn match_sorted(item_fn: &mut Item) -> syn::Result<()> {
    let item = if let Item::Fn(item) = item_fn {
        item
    } else {
        return Err(syn::Error::new(Span::call_site(), "expected fn"));
    };

    let mut visiter = MatchVisiter { res: None };

    visiter.visit_item_fn_mut(item);

    match visiter.res {
        Some(err) => Err(err),
        None => Ok(()),
    }
}
struct MatchVisiter {
    res: Option<syn::Error>,
}

impl VisitMut for MatchVisiter {
    fn visit_expr_match_mut(&mut self, expr: &mut crate::ExprMatch) {
        let old_len = expr.attrs.len();
        expr.attrs.retain(|attr| !attr.path().is_ident("sorted"));
        if old_len == expr.attrs.len() {
            return;
        }

        let mut arm_names: Vec<(String, &dyn ToTokens)> = Vec::with_capacity(expr.arms.len());
        for arm in expr.arms.iter() {
            match &arm.pat {
                Pat::Path(pat) => arm_names.push((get_path_string(&pat.path), &pat.path)),
                Pat::Struct(pat) => arm_names.push((get_path_string(&pat.path), &pat.path)),
                Pat::TupleStruct(pat) => arm_names.push((get_path_string(&pat.path), &pat.path)),
                Pat::Ident(pat_ident) => {
                    arm_names.push((pat_ident.ident.to_string(), &pat_ident.ident))
                }
                Pat::Wild(wild) => arm_names.push(("_".to_string(), wild)),
                _ => {
                    self.res = Some(syn::Error::new(arm.span(), "unsupported by #[sorted]"));
                    return;
                }
            }
        }

        let mut sorted = arm_names.clone();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));

        for (a, b) in sorted.iter().zip(arm_names.iter()) {
            if a.0 != b.0 {
                self.res = Some(syn::Error::new_spanned(
                    a.1,
                    format!("{} should sort before {}", a.0, b.0),
                ));
                return;
            }
        }

        self.visit_expr_match_mut(expr);
    }
}

fn get_path_string(path: &syn::Path) -> String {
    let mut string = String::new();
    for segemtn in path.segments.pairs() {
        let (seg, sep) = segemtn.into_tuple();
        string.push_str(&seg.ident.to_string());
        if let Some(sep) = sep {
            string.push_str(&quote! (#sep).to_string());
        }
    }
    string
}
