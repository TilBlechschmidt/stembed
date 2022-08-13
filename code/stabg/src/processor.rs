use crate::{
    context::ExecutionContext, identifier::ShortID, registry::TypeRegistry, Identifiable,
    Identifier,
};
use core::future::Future;

// TODO Document what happens when not all types are provided/registered, aka death :D

#[cfg(feature = "embedded")]
pub trait EmbeddedProcessor: Identifiable {
    /// List of types that will be retrieved from the context during execution
    const TYPES_INPUT: &'static [Identifier];

    /// List of types that will be pushed into the context during execution
    const TYPES_OUTPUT: &'static [Identifier];

    /// Estimated maximum size of the largest set of values that could be pushed onto the stack during execution.
    /// You should add four additional bytes per value due to internal overhead!
    ///
    /// Specifying an unrealistic usage in either direction may cause internal/external stack overflows where
    /// internal ones result in incomplete execution and a soft-crash while external ones could yield memory leaks
    /// and undefined behaviour.
    const STACK_USAGE: usize;

    type Fut<'s>: Future + 's
    where
        Self: 's;

    /// Triggers side-effects required prior to operation
    fn load(&mut self) -> Result<(), &'static str>;

    // TODO add execution errors (enum with ContextError or &'static str)
    fn process<'s>(&'s mut self, context: ExecutionContext) -> Self::Fut<'s>;

    /// Cleans up any side-effects caused by the `init` method
    fn unload(&mut self) {}
}

#[cfg(feature = "alloc")]
pub use self::alloc::*;

#[cfg(feature = "alloc")]
mod alloc {
    use super::*;
    use ::alloc::{boxed::Box, vec::Vec};

    pub(crate) type OwnedProcessor = Box<dyn Processor>;

    pub trait Processor {
        /// Globally unique identifier for this processor mostly used for debugging purposes
        fn identifier(&self) -> Identifier;

        /// Registers input & output types and triggers side-effects like spawning threads
        fn load(&mut self, context: &mut InitializationContext) -> Result<(), InitializationError>;

        // TODO add execution errors (enum with ContextError or BoxedError)
        fn process(&mut self, context: ExecutionContext);

        /// Cleans up any side-effects caused by the `init` method
        fn unload(&mut self) {}
    }

    #[derive(Debug)]
    pub enum InitializationError {
        TypeRegistrationFailed,
        InternalError(::alloc::string::String),
    }

    pub enum TypeUsage {
        Input,
        Output,
        InOut,
    }

    pub struct InitializationContext<'r> {
        type_registry: &'r mut TypeRegistry,
        pub(crate) input: Vec<ShortID>,
        pub(crate) output: Vec<ShortID>,
    }

    impl<'r> InitializationContext<'r> {
        pub(crate) fn new(type_registry: &'r mut TypeRegistry) -> Self {
            Self {
                type_registry,
                input: Vec::new(),
                output: Vec::new(),
            }
        }

        /// Informs the runtime that a given type will be used during execution and for what purpose
        pub fn register<T: Identifiable>(
            &mut self,
            usage: TypeUsage,
        ) -> Result<&mut Self, InitializationError> {
            self.register_raw(T::IDENTIFIER, usage)
        }

        /// Informs the runtime that a type with the given identifier will be used during execution and for what purpose
        pub fn register_raw(
            &mut self,
            id: Identifier,
            usage: TypeUsage,
        ) -> Result<&mut Self, InitializationError> {
            match self.type_registry.register(id) {
                Ok(id) => {
                    match usage {
                        TypeUsage::Input => self.input.push(id),
                        TypeUsage::Output => self.output.push(id),
                        TypeUsage::InOut => {
                            self.input.push(id);
                            self.output.push(id);
                        }
                    }
                    Ok(self)
                }
                Err(_) => {
                    // There is only one error case so no use in differentiating
                    Err(InitializationError::TypeRegistrationFailed)
                }
            }
        }
    }
}
