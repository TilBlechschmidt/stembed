use crate::Stroke;
use combine::parser::char::char;
use combine::stream::Stream;
use combine::{optional, Parser};

#[derive(PartialEq, Eq, Debug)]
enum MatchedEntry {
    Character,
    Number,
    None,
}

/// Optionally parses either a character or a number (depending on the second argument) and outputs whether or not and which input type was matched
fn steno_char<Input>(c: char, n: Option<char>) -> impl Parser<Input, Output = MatchedEntry>
where
    Input: Stream<Token = char>,
{
    let parser = if let Some(number) = n {
        optional(
            char(c)
                .map(|_| MatchedEntry::Character)
                .or(char(number).map(|_| MatchedEntry::Number)),
        )
        .left()
    } else {
        optional(char(c).map(|_| MatchedEntry::Character)).right()
    };

    parser.map(|v| v.unwrap_or(MatchedEntry::None))
}

/// Applies `steno_char` and then merges the result into a number-flag and a bit-shifted 32-bit number
fn steno_char_fold<Input>(
    previous: (bool, u32),
    c: char,
    n: Option<char>,
) -> impl Parser<Input, Output = (bool, u32)>
where
    Input: Stream<Token = char>,
{
    steno_char(c, n).map(move |current| match current {
        MatchedEntry::Character => (previous.0, previous.1 << 1 | 1),
        MatchedEntry::Number => (true, previous.1 << 1 | 1),
        MatchedEntry::None => (previous.0, previous.1 << 1),
    })
}

/// Matches characters and numbers present on the left half of the steno layout in order
fn left<Input>() -> impl Parser<Input, Output = u32>
where
    Input: Stream<Token = char>,
{
    let char = |c| move |previous| steno_char_fold(previous, c, None);
    let char_or_number = |c, n| move |previous| steno_char_fold(previous, c, Some(n));

    steno_char_fold((false, 0), '#', None)
        .then(char_or_number('S', '1'))
        .then(char_or_number('T', '2'))
        .then(char('K'))
        .then(char_or_number('P', '3'))
        .then(char('W'))
        .then(char_or_number('H', '4'))
        .then(char('R'))
        .map(|(n, v)| (v << 16) | ((n as u32) << 23))
}

/// Matches characters and numbers present in the middle half of the steno layout in order
fn middle<Input>() -> impl Parser<Input, Output = u32>
where
    Input: Stream<Token = char>,
{
    let char = |c| move |previous| steno_char_fold(previous, c, None);
    let char_or_number = |c, n| move |previous| steno_char_fold(previous, c, Some(n));

    steno_char_fold((false, 0), 'A', Some('5'))
        .then(char_or_number('O', '0'))
        .and(optional(combine::parser::char::char('-')))
        .map(|v| v.0)
        .then(char('*'))
        .then(char('E'))
        .then(char('U'))
        .map(|(n, v)| (v << 11) | ((n as u32) << 23))
}

/// Matches characters and numbers present on the right half of the steno layout in order
fn right<Input>() -> impl Parser<Input, Output = u32>
where
    Input: Stream<Token = char>,
{
    let char = |c| move |previous| steno_char_fold(previous, c, None);
    let char_or_number = |c, n| move |previous| steno_char_fold(previous, c, Some(n));

    steno_char_fold((false, 0), 'F', Some('6'))
        .then(char('R'))
        .then(char_or_number('P', '7'))
        .then(char('B'))
        .then(char_or_number('L', '8'))
        .then(char('G'))
        .then(char_or_number('T', '9'))
        .then(char('S'))
        .then(char('D'))
        .then(char('Z'))
        .map(|(n, v)| (v << 1) | ((n as u32) << 23))
}

/// Parses a stenography stroke, allows for omitting hyphens and replacement of characters with corresponding numbers
fn raw_stroke<Input>() -> impl Parser<Input, Output = u32>
where
    Input: Stream<Token = char>,
{
    let only_left = left();
    let left_right = left().and(char('-')).map(|v| v.0).and(right());
    let left_middle_right = left().and(middle()).and(right());
    let middle_right = middle().and(right());

    left_middle_right
        .map(|((l, m), r)| l | m | r)
        .or(left_right.map(|(l, r)| l | r))
        .or(middle_right.map(|(m, r)| m | r))
        .or(only_left)
}

/// Parses a stenography stroke, allows for omitting hyphens and replacement of characters with corresponding numbers
pub fn stroke<Input>() -> impl Parser<Input, Output = Stroke>
where
    Input: Stream<Token = char>,
{
    raw_stroke().map(Stroke::from)
}

#[cfg(test)]
mod does {
    use super::*;

    #[test]
    fn parse_full_stroke() {
        #[rustfmt::skip]
        assert_eq!(
            raw_stroke().parse("#STKPWHRAO*EUFRPBLGTSDZ"),
            Ok((0b11111111_11111_1111111111_0, ""))
                //#STKPWHR AO*EU FRPBLGTSDZ
        );
    }

    #[test]
    fn parse_left_only() {
        #[rustfmt::skip]
        assert_eq!(
            raw_stroke().parse("#STKPWHR"),
            Ok((0b11111111_00000_0000000000_0, ""))
                //#STKPWHR AO*EU FRPBLGTSDZ
        );
    }

    #[test]
    fn parse_right_only() {
        #[rustfmt::skip]
        assert_eq!(
            raw_stroke().parse("-FRPBLGTSDZ"),
            Ok((0b00000000_00000_1111111111_0, ""))
                //#STKPWHR AO*EU FRPBLGTSDZ
        );
    }

    #[test]
    fn parse_hyphenated() {
        #[rustfmt::skip]
        assert_eq!(
            raw_stroke().parse("H-P"),
            Ok((0b00000010_00000_0010000000_0, ""))
                //#STKPWHR AO*EU FRPBLGTSDZ
        );
    }

    #[test]
    fn parse_asterisk() {
        #[rustfmt::skip]
        assert_eq!(
            raw_stroke().parse("H*P"),
            Ok((0b00000010_00100_0010000000_0, ""))
                //#STKPWHR AO*EU FRPBLGTSDZ
        );
    }

    #[test]
    fn parse_numbers() {
        #[rustfmt::skip]
        assert_eq!(
            raw_stroke().parse("1234506789"),
            Ok((0b11101010_11000_1010101000_0, ""))
                //#STKPWHR AO*EU FRPBLGTSDZ
        );

        #[rustfmt::skip]
        assert_eq!(
            raw_stroke().parse("4-P"),
            Ok((0b10000010_00000_0010000000_0, ""))
                //#STKPWHR AO*EU FRPBLGTSDZ
        );
    }

    #[test]
    #[should_panic]
    #[ignore]
    // TODO Fix this, not critical for now but should be rejected
    fn fail_on_right_without_hyphen() {
        #[rustfmt::skip]
        assert_eq!(
            raw_stroke().parse("FRPBLGTSDZ"),
            Ok((0b00000000_00000_1111111111_0, ""))
                //#STKPWHR AO*EU FRPBLGTSDZ
        );
    }
}
