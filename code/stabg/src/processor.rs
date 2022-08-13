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

/// User-provided logic component — `The Heart Of The System` ❤️
///
/// This is the asynchronous and static equivalent to the regular [`Processor`](self::alloc::Processor),
/// optimized for use on embedded platforms that do not support `alloc` or `std`.
///
/// # Differences to the alloc-based Processor
///
/// Most changes are made to accommodate the restricted environment that microcontrollers present.
/// To prevent a whole class of on-device errors, a number of compile-time checks is executed.
/// For these to work, much information that is provided dynamically in case of the regular Processor
/// is required to be present as constants:
///
/// 1. Input & output types are provided statically through constants
///     - Allows compile-time checks of the execution order before you even flash the application
///     - Permits macro-based computation of the type registry size
/// 2. Stack usage estimation must be provided
///     - Since the [`Stack`](super::Stack) has to be [statically allocated](super::stack::FixedSizeStack), this is required to estimate its total size
///     - Allows automatic, compile-time stack size calculation based on the provided processors
///
/// Additionally, since blocking calculations are not permitted on embedded (they would interfere with other subsystems
/// like USB, Bluetooth, or other peripherals), the trait relies on [`Future`](core::future::Future)s instead of regular functions.
///
/// This is supported by the `generic_associated_types` and `type_alias_impl_trait` nightly feature which you may have to enable in order
/// to implement this trait! See the example below for more details.
///
/// # Example
///
/// In the future we will have full support for async traits! Though that is still some time out —
/// so until then you have to follow some additional steps to unleash the magic:
///
/// ```
/// #![feature(type_alias_impl_trait)]
/// #![feature(generic_associated_types)]
/// #
/// # use stabg::{error::ExecutionError, ExecutionContext, embedded::EmbeddedProcessor, Identifier, Identifiable};
/// # use core::future::Future;
///
/// #[derive(Identifiable)]
/// #[identifiable(name = "com.example.processor")]
/// struct ExampleProcessor;
///
/// impl EmbeddedProcessor for ExampleProcessor {
///     const TYPES_INPUT: &'static [Identifier] = &[];
///     const TYPES_OUTPUT: &'static [Identifier] = &[];
///     const STACK_USAGE: usize = 0;
///
///     // 1. Define an opaque, generic type for the Future you will return
///     type Fut<'s> = impl Future<Output = Result<(), ExecutionError>> + 's
///     where
///         Self: 's;
///
///     // 2. The type 'blank' is filled in by using it as the return type of your function
///     fn process<'s>(&'s mut self, context: ExecutionContext) -> Self::Fut<'s> {
///         async move {
///             Ok(())
///         }
///     }
///
///     // load & unload omitted for brevity
/// }
/// ```
#[cfg(feature = "nightly")]
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

    /// Allows you to trigger side-effects and make calculations before your processor is executed the first time
    fn load(&mut self) -> Result<(), &'static str> {
        Ok(())
    }

    /// Core logic of your processor that will be called each iteration cycle.
    /// Note that depending on the output of previous processors, it may run multiple times per cycle!
    fn process<'s>(&'s mut self, context: ExecutionContext) -> Self::Fut<'s>;

    /// Contains any cleanup required when your processor is removed. This may include side-effects caused in the [`load`](Self::load) function!
    fn unload(&mut self) {}
}

#[cfg(feature = "alloc")]
pub use self::alloc::*;

#[cfg(feature = "alloc")]
mod alloc {
    use crate::registry::RegistryError;

    use super::*;
    use ::alloc::vec::Vec;

    /// User-provided logic component — `The Heart Of The System` ❤️
    pub trait Processor {
        /// Globally unique identifier for this processor mostly used for debugging purposes
        fn identifier(&self) -> Identifier;

        /// This function is expected to register any types that will be used using the [`InitializationContext`](InitializationContext).
        /// You may trigger additional side-effects required for operation in this function.
        ///
        /// # ⚠️ Importance of registering types correctly
        ///
        /// The execution order of plugins is derived from the inputs & outputs they claim to use. Providing false or no information
        /// by not implementing this function according to the contents of your [`process`](Self::process) function **may result in your processor
        /// crashing or not executing at all!**
        fn load(&mut self, context: &mut InitializationContext) -> Result<(), InitializationError>;

        /// Core logic of your processor that will be called each iteration cycle.
        /// Note that depending on the output of previous processors, it may run multiple times per cycle!
        fn process(&mut self, context: ExecutionContext) -> Result<(), ExecutionError>;

        /// Contains any cleanup required when your processor is removed. This may include side-effects caused in the [`load`](Self::load) function!
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
    ///
    /// Used when registering types with the [`InitializationContext`](InitializationContext) in [`Processor::load`](Processor::load)
    pub enum TypeUsage {
        /// Values of the given type will only be *fetched from the stack*
        Input,
        /// Values of the given type will only be *pushed onto the stack*
        Output,
        /// Values of the given type will be *pushed onto & fetched from the stack*
        InOut,
    }

    /// Tool to inform the runtime what types a processor will use
    ///
    /// When your processor is first loaded, it will get an instance of this type.
    /// It is then expected to call [`register`](Self::register) for **every type** it will be fetching
    /// from or pushing onto the stack during execution. Failing to do so may initially seem to work,
    /// but can cause dependency issues down the road, especially in complex installations.
    ///
    /// This registration logic provides the information to the [`ProcessorCollection`](crate::desktop::ProcessorCollection)
    /// what values you depend on and can provide. It then derives an execution order from this information!
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
                Err(e) => Err(InitializationError::TypeRegistrationFailed(e)),
            }
        }
    }
}
