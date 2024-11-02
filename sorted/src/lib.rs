use proc_macro::{Span, TokenStream};
use syn::{parse_macro_input, spanned::Spanned, Item};

#[proc_macro_attribute]
pub fn sorted(args: TokenStream, input: TokenStream) -> TokenStream {
    let _ = args;
    let item = input.clone();
    let item = parse_macro_input!(item as syn::Item);
    let item = if let Item::Enum(item_enum) = item {
        item_enum
    } else {
        return syn::Error::new(
            Span::call_site().into(),
            "expected enum or match expression",
        )
        .into_compile_error()
        .into();
    };

    let idents = {
        let mut idents = item.variants.iter().collect::<Vec<_>>();
        idents.sort_by_key(|var| &var.ident);
        idents
    };
    for (sorted, origin) in idents.iter().zip(item.variants.iter()) {
        if sorted.ident != origin.ident {
            return syn::Error::new(
                sorted.span(),
                format!("{} should sort before {}", sorted.ident, origin.ident),
            )
            .into_compile_error()
            .into();
        }
    }

    input
}
