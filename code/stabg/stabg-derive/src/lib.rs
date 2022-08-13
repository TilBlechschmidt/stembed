use darling::FromDeriveInput;
use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[derive(FromDeriveInput, Default)]
#[darling(attributes(identifiable), forward_attrs(allow, doc, cfg))]
struct Opts {
    name: String,
}

/// This is a test
#[proc_macro_derive(Identifiable, attributes(identifiable))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);
    let Opts { name } = Opts::from_derive_input(&input).expect("Wrong options");
    let DeriveInput { ident, .. } = input;

    let output = quote! {
        impl Identifiable for #ident {
            const IDENTIFIER: Identifier = #name;
        }
    };

    output.into()
}
