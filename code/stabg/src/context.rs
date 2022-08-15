use serde::{de::DeserializeOwned, Serialize};

use crate::{
    registry::{Registry, ID_PROC_MARK, ID_VALUE_SET},
    serialization::Serializer,
    stack::{self, Stack},
    FixedSizeStack, Identifiable, Identifier, ShortID,
};

/// Errors caused by using the [`ExecutionContext`](ExecutionContext)'s API
#[derive(Debug)]
pub enum ExecutionContextError<E> {
    /// The requested or provided type has not been registered during plugin initialization.
    /// It is thus unknown to the system and was not accepted!
    UnknownType,
    /// The requested value is not present on the stack, though the type is known
    ValueNotFound,
    /// Transitive error caused by the underlying stack
    StackError(stack::StackError),
    /// Transitive error caused by the serialization logic
    SerializationError(E),
}

/// Constrained and simplified interface to a [`Stack`](Stack) for use in a processor
///
/// Not to be constructed by the user, an instance will be passed to processors by the [`Executor`](super::Executor).
/// Handles insertions of marker values required for the [`Executor`](super::Executor)
/// to correctly handle branching executions. This requires that the `Drop` implementation is
/// called orderly, using for example `mem::forget` will cause unexpected behaviour!
pub struct GenericExecutionContext<'s, 'r, S: Serializer + Copy> {
    stack: &'s mut dyn Stack,
    processor: ShortID,
    registry: &'r dyn Registry,
    serializer: S,
}

impl<'s, 'r, S: Serializer + Copy> GenericExecutionContext<'s, 'r, S> {
    /// Maximum number of bytes pushed onto the stack per instance
    // Contains PROC_MARK + VALUE_SET with the corresponding overhead from the stack
    pub const OVERHEAD: usize = (FixedSizeStack::<0>::OVERHEAD + (ShortID::BITS as usize / 8)) * 2;

    #[doc(hidden)]
    pub fn new(
        stack: &'s mut dyn Stack,
        processor: ShortID,
        registry: &'r dyn Registry,
        serializer: S,
    ) -> Self {
        Self {
            stack,
            processor,
            registry,
            serializer,
        }
    }

    pub fn get<T: Identifiable + DeserializeOwned>(
        &self,
    ) -> Result<T, ExecutionContextError<S::Error>> {
        self.serializer
            .deserialize(self.get_raw(T::IDENTIFIER)?)
            .map_err(ExecutionContextError::SerializationError)
    }

    pub fn push<T: Identifiable + Serialize>(
        &mut self,
        value: T,
    ) -> Result<&mut Self, ExecutionContextError<S::Error>> {
        self.serializer
            .clone()
            .serialize(&value, |buf| self.push_raw(T::IDENTIFIER, buf))
            .map(|_| self)
            .map_err(ExecutionContextError::SerializationError)
    }

    /// Fetches the latest value with the given type from the [`Stack`](Stack)
    fn get_raw(&self, id: Identifier) -> Result<&[u8], ExecutionContextError<S::Error>> {
        let code = self
            .registry
            .lookup(id)
            .ok_or(ExecutionContextError::UnknownType)?;

        if code == ID_VALUE_SET || code == ID_PROC_MARK {
            panic!("Registry returned reserved code");
        }

        self.stack
            .get(code)
            .ok_or(ExecutionContextError::ValueNotFound)
    }

    /// Pushes a new value of the given type onto the [`Stack`](Stack)
    fn push_raw(
        &mut self,
        id: Identifier,
        data: &[u8],
    ) -> Result<(), ExecutionContextError<S::Error>> {
        let code = self
            .registry
            .lookup(id)
            .ok_or(ExecutionContextError::UnknownType)?;

        if code == ID_VALUE_SET || code == ID_PROC_MARK {
            panic!("Registry returned reserved code");
        }

        self.stack
            .push(code, data)
            .map_err(ExecutionContextError::StackError)
    }

    // TODO This is a lacking explanation of the branching model! Create a larger write-up on the executor and link it.
    /// Creates a branching point, allowing multiple values of the same
    /// type to be processed or multiple different values processed in a particular order.
    /// Every following processor will be executed once for each of the provided values.
    ///
    /// After branching, it is not allowed to push further values. If you need this, open an issue and explain your use-case! 🙂
    pub fn branch(self) -> GenericExecutionBranch<'s, 'r, S> {
        GenericExecutionBranch::new(self)
    }
}

impl<'s, 'r, S: Serializer + Copy> Drop for GenericExecutionContext<'s, 'r, S> {
    /// Pushes a marker onto the stack letting the [`Executor`](super::Executor) know that
    /// the processor for which this context was created has finished executing and will no
    /// longer push additional values.
    fn drop(&mut self) {
        self.stack
            .push(ID_PROC_MARK, &self.processor.to_be_bytes())
            .expect("failed to push ProcessorBoundary marker");
    }
}

/// Version of [`ExecutionContext`](ExecutionContext) which creates an execution branch
///
/// For more details, see [`ExecutionContext::branch`](ExecutionContext::branch)!
pub struct GenericExecutionBranch<'s, 'r, S: Serializer + Copy> {
    context: GenericExecutionContext<'s, 'r, S>,
    value_count: u32,
}

impl<'s, 'r, S: Serializer + Copy> GenericExecutionBranch<'s, 'r, S> {
    fn new(context: GenericExecutionContext<'s, 'r, S>) -> Self {
        Self {
            context,
            value_count: 0,
        }
    }

    pub fn push_raw(
        &mut self,
        id: Identifier,
        data: &[u8],
    ) -> Result<(), ExecutionContextError<S::Error>> {
        self.context.push_raw(id, data)?;
        self.value_count += 1;
        Ok(())
    }
}

impl<'s, 'r, S: Serializer + Copy> Drop for GenericExecutionBranch<'s, 'r, S> {
    fn drop(&mut self) {
        self.context
            .stack
            .push(ID_VALUE_SET, &self.value_count.to_be_bytes())
            .expect("failed to push ValueSet marker");
    }
}
