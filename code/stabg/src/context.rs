use crate::{
    registry::TypeRegistry,
    stack::{self, Stack},
    Identifiable, Identifier, ShortID,
};

pub(crate) struct ValueSet(u8);
pub(crate) struct ProcessorBoundary(u8);

impl Identifiable for ValueSet {
    const IDENTIFIER: Identifier = "core.internal.valueSet";
}

impl Identifiable for ProcessorBoundary {
    const IDENTIFIER: Identifier = "core.internal.processorBoundary";
}

#[derive(Debug)]
pub enum ContextError {
    UnknownType,
    StackError(stack::Error),
}

#[allow(dead_code)]
pub struct ExecutionContext<'s, 'r> {
    stack: &'s mut dyn Stack,
    processor: ShortID,
    registry: &'r TypeRegistry,
}

impl<'s, 'r> ExecutionContext<'s, 'r> {
    pub(crate) fn new(
        stack: &'s mut dyn Stack,
        processor: ShortID,
        registry: &'r TypeRegistry,
    ) -> Self {
        debug_assert!(
            registry.contains(ValueSet::IDENTIFIER),
            "ValueSet type is not registered"
        );

        debug_assert!(
            registry.contains(ProcessorBoundary::IDENTIFIER),
            "ProcessorBoundary type is not registered"
        );

        Self {
            stack,
            processor,
            registry,
        }
    }

    /// Fetches the latest value with the given type from the stack
    pub fn get(&self, id: Identifier) -> Option<&[u8]> {
        self.stack.get(self.registry.lookup(id)?)
    }

    /// Pushes a new value of the given type onto the stack
    pub fn push(&mut self, id: Identifier, data: &[u8]) -> Result<&mut Self, ContextError> {
        self.stack
            .push(
                self.registry.lookup(id).ok_or(ContextError::UnknownType)?,
                data,
            )
            .map(|_| self)
            .map_err(ContextError::StackError)
    }

    /// Creates a branching point, allowing multiple values of the same
    /// type to be processed. Every following processor will be executed
    /// once for each of the provided values.
    pub fn branch(self) -> ExecutionBranch<'s, 'r> {
        ExecutionBranch::new(self)
    }
}

impl<'s, 'r> Drop for ExecutionContext<'s, 'r> {
    fn drop(&mut self) {
        let code = self.registry.lookup(ProcessorBoundary::IDENTIFIER).unwrap();
        self.stack
            // TODO Use ProcessorBoundary struct!
            .push(code, &[*self.processor])
            .expect("failed to push ProcessorBoundary marker");
    }
}

pub struct ExecutionBranch<'s, 'r> {
    context: ExecutionContext<'s, 'r>,
    value_count: u8,
}

impl<'s, 'r> ExecutionBranch<'s, 'r> {
    fn new(context: ExecutionContext<'s, 'r>) -> Self {
        Self {
            context,
            value_count: 0,
        }
    }

    pub fn push(&mut self, id: Identifier, data: &[u8]) -> Result<(), ContextError> {
        self.context.push(id, data)?;
        self.value_count += 1;
        Ok(())
    }
}

impl<'s, 'r> Drop for ExecutionBranch<'s, 'r> {
    fn drop(&mut self) {
        self.context
            // TODO Use ValueSet struct!
            .push(ValueSet::IDENTIFIER, &[self.value_count])
            .expect("failed to push ValueSet marker");
    }
}
