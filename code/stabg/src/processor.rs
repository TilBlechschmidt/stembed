//! Traits & types for user-provided logic components

use crate::{
    context::{ExecutionContext, ExecutionContextError},
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

/// User-provided logic component, asynchronous counterpart to the regular processor
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
/// so to save you all the boilerplating, we created a derive macro for you. Here is a minimal example on how to use it:
///
/// ```
/// #![feature(type_alias_impl_trait)]
/// #![feature(generic_associated_types)]
/// #
/// # use stabg::{processor::{ExecutionError, EmbeddedProcessor}, ExecutionContext, Identifier};
/// # use core::future::Future;
///
/// #[derive(Default, EmbeddedProcessor)]
/// struct ExampleProcessor;
///
/// impl ExampleProcessor {
///     async fn load(&mut self) -> Result<(), &'static str> {
///         Ok(())
///     }
///
///     async fn process(&mut self, mut ctx: ExecutionContext<'_, '_>) -> Result<(), ExecutionError> {
///         Ok(())
///     }
///
///     async fn unload(&mut self) {}
/// }
/// ```
///
/// The generated `impl EmbeddedProcessor` will call your load, unload, and process functions which are required to have the above method signature.
///
/// ## Type & stack usage annotations
///
/// While regular processors register their type usage at runtime, embedded processors are required to provide this information at compile time.
/// With the derive macro, you can add the `#[type_usage]` attribute to generate the necessary constants automatically.
///
/// Whenever your processor outputs values, it uses up memory of the stack. Since embedded devices require static memory allocation,
/// the memory usage of our processor needs to be known. You can add the `#[stack_usage]` attribute to provide this information.
/// In the example below, we expect to output one item and a total of two bytes.
///
/// Do note that the number of items and bytes are not related. Both independently describe the maximum number of each quantity
/// that your processor could potentially output. When in doubt, provide larger values. Too small of a value may crash your or other processors!
///
/// ```
/// # #![feature(type_alias_impl_trait)]
/// # #![feature(generic_associated_types)]
/// #
/// # use stabg::{processor::{ExecutionError, EmbeddedProcessor}, ExecutionContext, Identifier, Identifiable};
/// # use core::future::Future;
/// #
/// #[derive(Identifiable)]
/// #[identifier(name = "example.input.main")]
/// struct SomeInput(u8);
///
/// #[derive(Identifiable)]
/// #[identifier(name = "example.input.secondary")]
/// struct SecondaryInput(u8);
///
/// #[derive(Identifiable)]
/// #[identifier(name = "example.output.other")]
/// struct OtherOutput(u16);
///
/// #[derive(Default, EmbeddedProcessor)]
/// #[type_usage(
///     inputs(SomeInput, SecondaryInput),
///     outputs(OtherOutput)
/// )]
/// #[stack_usage(items = 1, bytes = 2)]
/// struct ExampleProcessor;
///
/// // `impl ExampleProcessor` omitted
/// # impl ExampleProcessor {
/// #     async fn load(&mut self) -> Result<(), &'static str> {
/// #         Ok(())
/// #     }
/// #
/// #     async fn process(&mut self, mut ctx: ExecutionContext<'_, '_>) -> Result<(), ExecutionError> {
/// #         Ok(())
/// #     }
/// #
/// #     async fn unload(&mut self) {}
/// # }
/// ```
#[cfg(feature = "nightly")]
pub trait EmbeddedProcessor {
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

    type LoadFut<'s>: Future<Output = Result<(), &'static str>> + 's
    where
        Self: 's;

    type ProcessFut<'s>: Future<Output = Result<(), ExecutionError>> + 's
    where
        Self: 's;

    type UnloadFut<'s>: Future<Output = ()> + 's
    where
        Self: 's;

    /// Allows you to trigger side-effects and make calculations before your processor is executed the first time
    fn load_raw<'s>(&'s mut self) -> Self::LoadFut<'s>;

    /// Core logic of your processor that will be called each iteration cycle.
    /// Note that depending on the output of previous processors, it may run multiple times per cycle!
    ///
    /// Usually you would not implement this method yourself but instead rely upon the derive macro. See the example for more details!
    fn process_raw<'s>(&'s mut self, context: ExecutionContext<'s, 's>) -> Self::ProcessFut<'s>;

    /// Contains any cleanup required when your processor is removed. This may include side-effects caused in the [`load`](Self::load) function!
    fn unload_raw<'s>(&'s mut self) -> Self::UnloadFut<'s>;
}

/// Abstracts away all the async trait boilerplating
///
/// For more details, usage instructions, and examples, take a look at the [`EmbeddedProcessor`](EmbeddedProcessor) traits documentation!
#[cfg(all(feature = "derive", feature = "nightly"))]
pub use stabg_derive::EmbeddedProcessor;

#[cfg(feature = "alloc")]
pub use self::alloc::*;

#[cfg(feature = "alloc")]
mod alloc {
    use super::*;
    use ::alloc::vec::Vec;

    /// User-provided logic component — `The Heart Of The System` ❤️
    #[doc(cfg(feature = "alloc"))]
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
        fn load(
            &mut self,
            context: &mut InitializationContext,
        ) -> Result<(), ::alloc::string::String>;

        /// Core logic of your processor that will be called each iteration cycle.
        /// Note that depending on the output of previous processors, it may run multiple times per cycle!
        fn process(&mut self, context: ExecutionContext) -> Result<(), ExecutionError>;

        /// Contains any cleanup required when your processor is removed. This may include side-effects caused in the [`load`](Self::load) function!
        fn unload(&mut self) {}
    }

    /// Purposes for which a [`Processor`](Processor) will use a type
    ///
    /// Used when registering types with the [`InitializationContext`](InitializationContext) in [`Processor::load`](Processor::load)
    #[doc(cfg(feature = "alloc"))]
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
    #[doc(cfg(feature = "alloc"))]
    pub struct InitializationContext {
        pub(crate) input: Vec<Identifier>,
        pub(crate) output: Vec<Identifier>,
    }

    impl InitializationContext {
        pub(crate) fn new() -> Self {
            Self {
                input: Vec::new(),
                output: Vec::new(),
            }
        }

        /// Informs the runtime that a given type will be used during execution and for what purpose
        pub fn register<T: Identifiable>(&mut self, usage: TypeUsage) -> &mut Self {
            self.register_raw(T::IDENTIFIER, usage)
        }

        /// Informs the runtime that a type with the given identifier will be used during execution and for what purpose
        pub fn register_raw(&mut self, id: Identifier, usage: TypeUsage) -> &mut Self {
            match usage {
                TypeUsage::Input => self.input.push(id),
                TypeUsage::Output => self.output.push(id),
                TypeUsage::InOut => {
                    self.input.push(id);
                    self.output.push(id);
                }
            }

            self
        }
    }
}
