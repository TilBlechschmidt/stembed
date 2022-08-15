use darling::{util::PathList, Error, FromDeriveInput};
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

#[derive(FromDeriveInput, Default)]
#[darling(attributes(stack_usage), forward_attrs(allow, doc, cfg))]
struct StackUsageOpts {
    #[darling(default)]
    items: usize,
}

#[derive(FromDeriveInput, Default)]
#[darling(attributes(type_usage), forward_attrs(allow, doc, cfg))]
struct TypeUsageOpts {
    #[darling(default)]
    inputs: PathList,
    #[darling(default)]
    outputs: PathList,
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

#[proc_macro_derive(EmbeddedProcessor, attributes(stack_usage, type_usage))]
pub fn derive_embedded_processor(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);
    let mut errors = Error::accumulator();

    let items = match StackUsageOpts::from_derive_input(&input) {
        Ok(StackUsageOpts { items }) => items,
        Err(err) => {
            errors.push(err);
            0
        }
    };

    let (inputs, outputs) = match TypeUsageOpts::from_derive_input(&input) {
        Ok(TypeUsageOpts { inputs, outputs }) => (inputs, outputs),
        Err(err) => {
            errors.push(err);
            (
                PathList::new(Vec::<syn::Path>::new()),
                PathList::new(Vec::<syn::Path>::new()),
            )
        }
    };

    let DeriveInput { ident, .. } = input;

    if !outputs.is_empty() && items == 0 {
        errors.push(Error::custom(
            "Output types defined but `stack_usage` is empty or not present",
        ));
    } else if outputs.is_empty() && items > 0 {
        errors.push(Error::custom(
            "No output types defined but `stack_usage` is larger than `0`",
        ));
    }

    let output = quote! {
        impl ::stabg::processor::EmbeddedProcessor for #ident {
            const TYPES_INPUT: &'static [::stabg::Identifier] = &[#(<#inputs>::IDENTIFIER, )*];
            const TYPES_OUTPUT: &'static [::stabg::Identifier] = &[#(<#outputs>::IDENTIFIER, )*];
            const STACK_USAGE: usize = ::stabg::determine_stack_usage(#items, &[
                #(::core::mem::size_of::<#outputs>(), )*
            ]);

            type LoadFut<'s> = impl ::core::future::Future<Output = Result<(), &'static str>> + 's
            where
                Self: 's;

            type ProcessFut<'s> = impl ::core::future::Future<Output = Result<(), ::stabg::processor::EmbeddedExecutionError>> + 's
            where
                Self: 's;

            type UnloadFut<'s> = impl ::core::future::Future<Output = ()> + 's
            where
                Self: 's;

            fn load_raw<'s>(&'s mut self) -> Self::LoadFut<'s> {
                async move { self.load().await }
            }

            fn process_raw<'s>(&'s mut self, context: ::stabg::processor::EmbeddedExecutionContext<'s, 's>) -> Self::ProcessFut<'s> {
                async move { self.process(context).await }
            }

            fn unload_raw<'s>(&'s mut self) -> Self::UnloadFut<'s> {
                async move { self.unload().await }
            }
        }
    };

    if let Err(error) = errors.finish() {
        let errors = error.write_errors();
        quote! {
            #output
            #errors
        }
        .into()
    } else {
        output.into()
    }
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

    let processor_count = processor_type.len();

    let output = quote! {
        #[automatically_derived]
        impl ::stabg::AsyncExecutionQueue for #ident {
            const PROCESSOR_COUNT: usize = #processor_count;
            const STACK_USAGE: usize = #(<#processor_type>::STACK_USAGE + )* ::stabg::processor::EmbeddedExecutionContext::OVERHEAD * Self::PROCESSOR_COUNT;

            type Fut<'s> = impl ::core::future::Future<Output = Result<(), ::stabg::processor::EmbeddedExecutionError>> + 's
            where
                Self: 's;

            fn run<'s>(&'s mut self, start_id: Option<ShortID>, stack: &'s mut dyn Stack) -> Self::Fut<'s> {
                async move {
                    let types = ::core::iter::empty();
                    #(
                        let types = types.chain(<#processor_type>::TYPES_INPUT.iter());
                        let types = types.chain(<#processor_type>::TYPES_OUTPUT.iter());
                    )*

                    let registry = ::stabg::IteratorRegistry(types);
                    let serializer = unsafe { ::stabg::serialization::TransmuteSerializer::new() };

                    let mut id: ShortID = 0;
                    let mut running = start_id.is_none();

                    #(
                        if !running && Some(id) == start_id {
                            running = true;
                        }

                        if running {
                            let context = ::stabg::processor::EmbeddedExecutionContext::new(stack, id, &registry, serializer);
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
