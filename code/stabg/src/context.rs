use crate::{
    registry::{Registry, ID_PROC_MARK, ID_VALUE_SET},
    stack::{self, Stack},
    Identifier, ShortID,
};

/// Errors caused by using the [`ExecutionContext`](ExecutionContext)'s API
#[derive(Debug)]
pub enum ExecutionContextError {
    /// The requested or provided type has not been registered during plugin initialization.
    /// It is thus unknown to the system and was not accepted!
    UnknownType,
    /// The requested value is not present on the stack, though the type is known.
    ValueNotFound,
    /// Other transitive error caused by the underlying stack.
    StackError(stack::StackError),
}

/// Constrained and simplified interface to a [`Stack`](Stack) for use in a processor
///
/// Not to be constructed by the user, an instance will be passed to processors by the [`Executor`](super::Executor).
/// Handles insertions of marker values required for the [`Executor`](super::Executor)
/// to correctly handle branching executions. This requires that the `Drop` implementation is
/// called orderly, using for example `mem::forget` will cause unexpected behaviour!
pub struct ExecutionContext<'s, 'r> {
    stack: &'s mut dyn Stack,
    processor: ShortID,
    registry: &'r dyn Registry,
}

impl<'s, 'r> ExecutionContext<'s, 'r> {
    #[doc(hidden)]
    pub fn new(stack: &'s mut dyn Stack, processor: ShortID, registry: &'r dyn Registry) -> Self {
        Self {
            stack,
            processor,
            registry,
        }
    }

    /// Fetches the latest value with the given type from the [`Stack`](Stack)
    pub fn get(&self, id: Identifier) -> Result<&[u8], ExecutionContextError> {
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
    pub fn push(
        &mut self,
        id: Identifier,
        data: &[u8],
    ) -> Result<&mut Self, ExecutionContextError> {
        let code = self
            .registry
            .lookup(id)
            .ok_or(ExecutionContextError::UnknownType)?;

        if code == ID_VALUE_SET || code == ID_PROC_MARK {
            panic!("Registry returned reserved code");
        }

        self.stack
            .push(code, data)
            .map(|_| self)
            .map_err(ExecutionContextError::StackError)
    }

    // TODO This is a lacking explanation of the branching model! Create a larger write-up on the executor and link it.
    /// Creates a branching point, allowing multiple values of the same
    /// type to be processed or multiple different values processed in a particular order.
    /// Every following processor will be executed once for each of the provided values.
    ///
    /// After branching, it is not allowed to push further values. If you need this, open an issue and explain your use-case! 🙂
    pub fn branch(self) -> ExecutionBranch<'s, 'r> {
        ExecutionBranch::new(self)
    }
}

impl<'s, 'r> Drop for ExecutionContext<'s, 'r> {
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
pub struct ExecutionBranch<'s, 'r> {
    context: ExecutionContext<'s, 'r>,
    value_count: u32,
}

impl<'s, 'r> ExecutionBranch<'s, 'r> {
    fn new(context: ExecutionContext<'s, 'r>) -> Self {
        Self {
            context,
            value_count: 0,
        }
    }

    pub fn push(&mut self, id: Identifier, data: &[u8]) -> Result<(), ExecutionContextError> {
        self.context.push(id, data)?;
        self.value_count += 1;
        Ok(())
    }
}

impl<'s, 'r> Drop for ExecutionBranch<'s, 'r> {
    fn drop(&mut self) {
        self.context
            .stack
            .push(ID_VALUE_SET, &self.value_count.to_be_bytes())
            .expect("failed to push ValueSet marker");
    }
}
