use crate::{
    context::{ExecutionContext, ExecutionContextError},
    identifier::ShortID,
    registry::Registry,
    Identifiable, Identifier,
};
use core::future::Future;

// TODO Document what happens when not all types are provided/registered, aka death :D

/// Errors encountered while executing a processor
#[derive(Debug)]
pub enum ExecutionError {
    /// Transitive errors caused by the execution context passed into the processor during execution.
    ContextError(ExecutionContextError),

    /// Unknown internal error from within the plugin, unrelated to the execution context.
    /// Turns into a `String` when the `alloc` feature is enabled!
    #[cfg(not(feature = "alloc"))]
    InternalError(&'static str),

    /// Unknown internal error from within the plugin, unrelated to the execution context.
    /// Turns into a `&'static str` when the `alloc` feature is not enabled!
    #[cfg(feature = "alloc")]
    InternalError(::alloc::string::String),
}

impl From<ExecutionContextError> for ExecutionError {
    fn from(e: ExecutionContextError) -> Self {
        Self::ContextError(e)
    }
}

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

    type Fut<'s>: Future<Output = Result<(), ExecutionError>> + 's
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
    use crate::registry::RegistryError;

    use super::*;
    use ::alloc::{boxed::Box, vec::Vec};

    pub(crate) type OwnedProcessor = Box<dyn Processor>;

    pub trait Processor {
        /// Globally unique identifier for this processor mostly used for debugging purposes
        fn identifier(&self) -> Identifier;

        /// Registers input & output types and triggers side-effects like spawning threads
        fn load(&mut self, context: &mut InitializationContext) -> Result<(), InitializationError>;

        // TODO add execution errors (enum with ContextError or BoxedError)
        fn process(&mut self, context: ExecutionContext) -> Result<(), ExecutionError>;

        /// Cleans up any side-effects caused by the `init` method
        fn unload(&mut self) {}
    }

    /// Errors caused while initializing a dynamic [`Processor`](Processor)
    #[derive(Debug)]
    pub enum InitializationError {
        /// The type could not be registered due to a transitive error in the underlying [`Registry`](Registry)
        TypeRegistrationFailed(RegistryError),
        /// Unknown internal error from within the plugin, unrelated to the initialization context.
        InternalError(::alloc::string::String),
    }

    /// Purposes for which a [`Processor`](Processor) will use a type
    pub enum TypeUsage {
        Input,
        Output,
        InOut,
    }

    pub struct InitializationContext<'r> {
        type_registry: &'r mut dyn Registry,
        pub(crate) input: Vec<ShortID>,
        pub(crate) output: Vec<ShortID>,
    }

    impl<'r> InitializationContext<'r> {
        pub(crate) fn new(type_registry: &'r mut dyn Registry) -> Self {
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
                Err(e) => {
                    Err(InitializationError::TypeRegistrationFailed(e))
                }
            }
        }
    }
}
