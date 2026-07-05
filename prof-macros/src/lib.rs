use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn prof(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = input_fn.sig.ident.to_string();
    let attrs = &input_fn.attrs;
    let vis = &input_fn.vis;
    let sig = &input_fn.sig;
    let block = &input_fn.block;

    let expanded = quote! {
        #(#attrs)*
        #vis #sig {
            let _guard = crate::prof::prof_guard(concat!(module_path!(), "::", #fn_name));
            #block
        }
    };
    expanded.into()
}
