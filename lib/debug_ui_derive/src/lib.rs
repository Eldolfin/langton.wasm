use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Config, attributes(field))]
pub fn derive_config(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    // For now, just implement a dummy trait to prove macro works
    let expanded = quote! {
        impl #name {
            pub fn debug_ui_config() {
                // TODO: implement config UI logic
            }
        }
    };
    TokenStream::from(expanded)
}
