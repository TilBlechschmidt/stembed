use super::{OutputCommand, OutputProcessor};
use alloc::string::String;
use core::ops::Deref;

/// Simple tool for combining [`OutputCommands`](super::OutputCommand) in-memory into a string
pub struct OutputAggregator(String);

impl OutputAggregator {
    pub fn new() -> Self {
        Self(String::new())
    }
}

impl OutputProcessor for OutputAggregator {
    fn apply<I: Iterator<Item = char>>(&mut self, command: OutputCommand<I>) {
        match command {
            OutputCommand::Backspace(n) => {
                for _ in 0..n {
                    self.0.pop();
                }
            }
            OutputCommand::Write(string) => {
                for c in string {
                    self.0.push(c);
                }
            }
        }
    }
}

impl Default for OutputAggregator {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for OutputAggregator {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
