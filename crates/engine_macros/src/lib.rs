use darling::{FromDeriveInput, FromField};
use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[derive(FromDeriveInput)]
#[darling(attributes(param), supports(struct_named))]
struct ConfigOpts {
    ident: syn::Ident,
    data: darling::ast::Data<(), ParamFieldOpts>,
}

#[derive(FromField)]
#[darling(attributes(param))]
struct ParamFieldOpts {
    ident: Option<syn::Ident>,
    name: String,
    default: String,
    #[darling(default)]
    range: Option<String>,
    #[darling(default)]
    step: Option<f64>,
    #[darling(default)]
    scale: Option<String>,
    #[darling(default)]
    needs_restart: bool,
    #[darling(default)]
    section: Option<String>,
}

fn parse_range_tokens(range_str: &str) -> proc_macro2::TokenStream {
    range_str
        .parse()
        .unwrap_or_else(|e| panic!("invalid range expression in #[param(range = ...)]: {e}"))
}

fn parse_default_tokens(default_str: &str) -> proc_macro2::TokenStream {
    default_str
        .parse()
        .unwrap_or_else(|e| panic!("invalid default expression in #[param(default = ...)]: {e}"))
}

#[proc_macro_derive(SimulationConfig, attributes(param))]
pub fn derive_simulation_config(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let opts = match ConfigOpts::from_derive_input(&input) {
        Ok(v) => v,
        Err(e) => return e.write_errors().into(),
    };

    let struct_name = &opts.ident;
    let fields = opts
        .data
        .take_struct()
        .expect("SimulationConfig can only be derived for structs with named fields");

    let mut section_stmts: Vec<proc_macro2::TokenStream> = vec![];
    let mut field_inits: Vec<proc_macro2::TokenStream> = vec![];

    for field in fields.iter() {
        let field_name = field.ident.as_ref().unwrap();
        let name = &field.name;
        let default_val = parse_default_tokens(&field.default);

        if let Some(section) = &field.section {
            section_stmts.push(quote! {
                debug_ui.start_section(#section);
            });
        }

        let range_expr = field.range.as_ref().map(|r| {
            let tokens = parse_range_tokens(r);
            quote! { range: #tokens, }
        });

        let step_expr = field.step.map(|s| {
            quote! { step_size: #s, }
        });

        let scale_expr = field.scale.as_ref().map(|s| {
            let scale_ident = syn::Ident::new(s, proc_macro2::Span::mixed_site());
            quote! { scale: debug_ui::Scale::#scale_ident, }
        });

        let restart_expr = if field.needs_restart {
            quote! { needs_restart: true, }
        } else {
            quote! {}
        };

        section_stmts.push(quote! {
            let #field_name = debug_ui.param(debug_ui::ParamParam {
                name: #name,
                default_value: #default_val,
                #range_expr
                #step_expr
                #scale_expr
                #restart_expr
                ..Default::default()
            });
        });

        field_inits.push(quote! { #field_name });
    }

    let expanded = quote! {
        impl #struct_name {
            pub fn new(debug_ui: &mut debug_ui::DebugUI) -> Self {
                #(#section_stmts)*
                Self {
                    #(#field_inits),*
                }
            }
        }
    };

    expanded.into()
}
