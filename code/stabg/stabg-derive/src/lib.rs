use darling::FromDeriveInput;
use proc_macro::{self, TokenStream};
use proc_macro_error::abort;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[derive(FromDeriveInput, Default)]
#[darling(attributes(identifier), forward_attrs(allow, doc, cfg))]
struct IdentifiableOpts {
    name: String,
    version: Option<String>,
}

#[proc_macro_derive(Identifiable, attributes(identifier))]
pub fn derive_identifiable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);
    let IdentifiableOpts { name, version } =
        IdentifiableOpts::from_derive_input(&input).expect("Wrong options");
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

#[proc_macro_derive(AsyncExecutionQueue)]
#[proc_macro_error::proc_macro_error]
pub fn derive_async_execution_queue(input: TokenStream) -> TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input);

    let mut processor_type = Vec::new();
    let mut processor_ident = Vec::new();

    match data {
        Data::Struct(s) => {
            match s.fields {
                Fields::Named(f) => {
                    for field in f.named.into_iter() {
                        processor_type.push(field.ty);
                        processor_ident.push(field.ident.unwrap());
                    }
                }
                Fields::Unnamed(_) => {
                    abort!(s.fields, "Deriving `AsyncExecutionQueue` for structs with unnamed fields is unsupported");
                }
                Fields::Unit => {
                    abort!(
                        s.fields,
                        "Struct is missing fields implementing `EmbeddedProcessor`"
                    );
                }
            }
        }
        Data::Enum(e) => {
            abort!(
                e.enum_token,
                "Deriving `AsyncExecutionQueue` for enums is unsupported"
            );
        }
        Data::Union(u) => {
            abort!(
                u.union_token,
                "Deriving `AsyncExecutionQueue` for unions is unsupported"
            );
        }
    }

    let output = quote! {
        #[automatically_derived]
        impl ::stabg::AsyncExecutionQueue for #ident {
            const STACK_USAGE: usize = #(#processor_type::STACK_USAGE + )* 0;

            type Fut<'s> = impl ::core::future::Future<Output = Result<(), ExecutionError>> + 's
            where
                Self: 's;

            fn run<'s>(&'s mut self, start_id: Option<ShortID>, stack: &'s mut dyn Stack) -> Self::Fut<'s> {
                async move {
                    let types = ::core::iter::empty();
                    #(
                        let types = types.chain(#processor_type::TYPES_INPUT.iter());
                        let types = types.chain(#processor_type::TYPES_OUTPUT.iter());
                    )*

                    let registry = ::stabg::IteratorRegistry(types);

                    let mut id: ShortID = 0;
                    let mut running = start_id.is_none();

                    #(
                        if !running && Some(id) == start_id {
                            running = true;
                        }

                        if running {
                            let context = ::stabg::ExecutionContext::new(stack, id, &registry);
                            self.#processor_ident.process(context).await?;
                        }

                        id += 1;
                    )*

                    Ok(())
                }
            }
        }
    };

    output.into()
}
