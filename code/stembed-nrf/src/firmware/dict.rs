use core::marker::PhantomData;

use alloc::string::ToString;
use futures::Future;
use smallvec::smallvec;
use stembed::core::{
    dict::Dictionary, engine::Command, processor::text_formatter::TextOutputCommand, Stroke,
};

#[derive(Default)]
pub struct DummyDictionary<'c> {
    phantom: PhantomData<&'c ()>,
}

impl<'c> DummyDictionary<'c> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<'c> Dictionary for DummyDictionary<'c> {
    type Stroke = Stroke<'c>;
    type OutputCommand = TextOutputCommand;
    type LookupFuture<'a> = impl Future<Output = Option<stembed::core::dict::CommandList<Self::OutputCommand>>> + 'a
    where
        Self: 'a;

    fn lookup<'a>(&'a self, _outline: &'a [Self::Stroke]) -> Self::LookupFuture<'a> {
        async move { None }
    }

    fn fallback_commands(
        &self,
        stroke: &Self::Stroke,
    ) -> stembed::core::dict::CommandList<Self::OutputCommand> {
        let formatted_stroke = stroke.to_string();
        let command = Command::Output(TextOutputCommand::Write(formatted_stroke));
        smallvec![command]
    }

    fn longest_outline_length(&self) -> usize {
        16
    }
}
