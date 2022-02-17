use alloc::string::String;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TextOutputInstruction {
    Backspace(usize),
    Write(String),
}
