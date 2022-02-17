use super::OutputSink;
use crate::core::processor::{text_formatter::TextOutputInstruction, OutputInstructionSet};
use autopilot::key::{tap, type_string, Code, KeyCode};

pub struct OSOutput;

impl OutputSink for OSOutput {
    type Instruction = TextOutputInstruction;

    fn send(&mut self, output: OutputInstructionSet<Self::Instruction>) {
        for instruction in output {
            match instruction {
                TextOutputInstruction::Backspace(count) => {
                    for _ in 0..count {
                        tap(&Code(KeyCode::Backspace), &[], 0, 0);
                    }
                }
                TextOutputInstruction::Write(text) => type_string(&text, &[], 0., 0.),
            }
        }
    }
}
