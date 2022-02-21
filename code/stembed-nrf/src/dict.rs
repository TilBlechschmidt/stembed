use core::marker::PhantomData;

use alloc::string::ToString;
use smallvec::smallvec;
use stembed::core::{
    dict::Dictionary, engine::Command, processor::text_formatter::TextOutputCommand, Stroke,
};

#[derive(Default)]
pub struct DummyDictionary<'c> {
    phantom: PhantomData<&'c ()>,
}

impl<'c> Dictionary for DummyDictionary<'c> {
    type Stroke = Stroke<'c>;
    type OutputCommand = TextOutputCommand;

    fn lookup(
        &self,
        _outline: &[Self::Stroke],
    ) -> Option<stembed::core::dict::CommandList<Self::OutputCommand>> {
        None
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
        1
    }
}
