use super::{OutputCommand, OutputProcessor};

mod keyinput;
use keyinput::*;

pub struct OSOutput;

impl OSOutput {
    pub fn new() -> Self {
        Self
    }
}

impl OutputProcessor for OSOutput {
    fn apply<I: Iterator<Item = char>>(&mut self, command: OutputCommand<I>) {
        match command {
            OutputCommand::Backspace(n) => {
                for _ in 0..n {
                    tap(&Code(KeyCode::Backspace), &[]);
                }
            }
            OutputCommand::Write(string) => {
                for c in string {
                    tap(&Character(c), &[]);
                }
            }
        }
    }
}

impl Default for OSOutput {
    fn default() -> Self {
        Self::new()
    }
}
