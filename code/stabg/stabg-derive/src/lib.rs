use darling::FromDeriveInput;
use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[derive(FromDeriveInput, Default)]
#[darling(attributes(identifiable), forward_attrs(allow, doc, cfg))]
struct Opts {
    name: String,
    version: Option<String>,
}

#[proc_macro_derive(Identifiable, attributes(identifiable))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);
    let Opts { name, version } = Opts::from_derive_input(&input).expect("Wrong options");
    let DeriveInput { ident, .. } = input;

    let content = match version {
        Some(v) => {
            quote! {
                const IDENTIFIER: Identifier = concat!(#name, "-", #v);
            }
        }
        None => quote! {
            const IDENTIFIER: Identifier = #name;
        },
    };

    let output = quote! {
        impl Identifiable for #ident {
            #content
        }
    };

    output.into()
}
