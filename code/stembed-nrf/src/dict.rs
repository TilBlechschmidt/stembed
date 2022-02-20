use alloc::string::ToString;
use stembed::core::{dict::Dictionary, Stroke, processor::text_formatter::TextOutputCommand, engine::Command};
use smallvec::smallvec;

pub struct DummyDictionary;

impl Dictionary for DummyDictionary {
    type Stroke = Stroke;
    type OutputCommand = TextOutputCommand;

    fn lookup(
        &mut self,
        outline: &[Self::Stroke],
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