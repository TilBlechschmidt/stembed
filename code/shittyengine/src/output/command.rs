pub enum OutputCommand<CharIter: Iterator<Item = char>> {
    Backspace(u8),
    Write(CharIter),
}
