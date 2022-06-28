use super::stroke;
use crate::formatter::{AttachmentMode, CapitalizationMode, FormatterCommand};
use crate::Stroke;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use combine::{
    any, attempt, between, choice, eof,
    error::Commit,
    many, many1, one_of, optional,
    parser::{
        char::{char, spaces, string},
        function,
    },
    produce, satisfy_map, sep_by1, ParseError, Parser, Stream,
};
use core::fmt::{Display, Write};
use core::ops::Deref;

type OwnedFormatterCommand = FormatterCommand<String>;

#[repr(transparent)]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct CommandList(pub Vec<OwnedFormatterCommand>);

impl From<Vec<OwnedFormatterCommand>> for CommandList {
    fn from(commands: Vec<OwnedFormatterCommand>) -> Self {
        Self(commands)
    }
}

impl Deref for CommandList {
    type Target = Vec<OwnedFormatterCommand>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Heap allocated list of strokes
#[derive(Default, Clone)]
pub struct Outline(Vec<crate::Stroke>);

impl Outline {
    pub fn into_bytes(self) -> impl Iterator<Item = u8> {
        self.0.into_iter().flat_map(Stroke::into_bytes)
    }
}

impl Display for Outline {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for (i, stroke) in self.0.iter().enumerate() {
            stroke.fmt(f)?;

            if i < self.0.len() - 1 {
                f.write_char('/')?;
            }
        }

        Ok(())
    }
}

impl Deref for Outline {
    type Target = Vec<crate::Stroke>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Extend<Stroke> for Outline {
    fn extend<T: IntoIterator<Item = Stroke>>(&mut self, iter: T) {
        self.0.extend(iter)
    }
}

fn lex<Input, P>(p: P) -> impl Parser<Input, Output = P::Output>
where
    P: Parser<Input>,
    Input: Stream<Token = char>,
{
    p.skip(spaces())
}

fn json_char<Input>() -> impl Parser<Input, Output = char>
where
    Input: Stream<Token = char>,
{
    function::parser(|input: &mut Input| {
        let (c, committed) = any().parse_lazy(input).into_result()?;

        let mut back_slash_char = satisfy_map(|c| {
            Some(match c {
                '"' => '"',
                '\\' => '\\',
                '/' => '/',
                'b' => '\u{0008}',
                'f' => '\u{000c}',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                _ => return None,
            })
        });

        match c {
            '\\' => committed.combine(|_| back_slash_char.parse_stream(input).into_result()),
            '"' => Err(Commit::Peek(Input::Error::empty(input.position()).into())),
            _ => Ok((c, committed)),
        }
    })
}

fn outline<Input>() -> impl Parser<Input, Output = Outline>
where
    Input: Stream<Token = char>,
{
    between(char('"'), lex(char('"')), sep_by1(stroke(), char('/'))).expected("outline")
}

// TODO This one does basically the same as json_char, merge them into one parametric parser
fn translation_char<Input>() -> impl Parser<Input, Output = char>
where
    Input: Stream<Token = char>,
{
    function::parser(|input: &mut Input| {
        let (c, committed) = json_char().parse_lazy(input).into_result()?;

        let mut back_slash_char = satisfy_map(|c| {
            Some(match c {
                '{' => '{',
                '}' => '}',
                '^' => '^',
                _ => return None,
            })
        });

        match c {
            '\\' => committed.combine(|_| back_slash_char.parse_stream(input).into_result()),
            '{' | '}' => Err(Commit::Peek(Input::Error::empty(input.position()).into())),
            _ => Ok((c, committed)),
        }
    })
}

fn translation_text<'s, Input>() -> impl Parser<Input, Output = OwnedFormatterCommand>
where
    Input: Stream<Token = char>,
{
    many1(translation_char()).map(OwnedFormatterCommand::Write)
}

fn meta_operator_item<Input>() -> impl Parser<Input, Output = CommandList>
where
    Input: Stream<Token = char>,
{
    choice((
        char('^').map(|_| {
            vec![OwnedFormatterCommand::ChangeAttachment(
                AttachmentMode::Next,
            )]
            .into()
        }),
        char('>').map(|_| {
            vec![OwnedFormatterCommand::ChangeCapitalization(
                CapitalizationMode::LowercaseNext,
            )]
            .into()
        }),
        char('<').map(|_| {
            vec![OwnedFormatterCommand::ChangeCapitalization(
                CapitalizationMode::UppercaseNext,
            )]
            .into()
        }),
        attempt(string("-|")).map(|_| {
            vec![OwnedFormatterCommand::ChangeCapitalization(
                CapitalizationMode::CapitalizeNext,
            )]
            .into()
        }),
        char(',').map(|punctuation: char| {
            vec![
                OwnedFormatterCommand::ChangeAttachment(AttachmentMode::Next),
                OwnedFormatterCommand::Write(punctuation.into()),
            ]
            .into()
        }),
        one_of(".:;!?".chars()).map(|punctuation: char| {
            vec![
                OwnedFormatterCommand::ChangeAttachment(AttachmentMode::Next),
                OwnedFormatterCommand::Write(punctuation.into()),
                OwnedFormatterCommand::ChangeCapitalization(CapitalizationMode::CapitalizeNext),
            ]
            .into()
        }),
        translation_text()
            .map(|c| vec![c].into())
            .expected("string"),
    ))
}

fn meta_operator_glue<'s, Input>() -> impl Parser<Input, Output = CommandList>
where
    Input: Stream<Token = char>,
{
    char('&').then(|_| {
        translation_text()
            .map(|c| {
                vec![
                    OwnedFormatterCommand::ChangeAttachment(AttachmentMode::Glue),
                    c,
                    OwnedFormatterCommand::ChangeAttachment(AttachmentMode::Glue),
                ]
                .into()
            })
            .expected("string")
    })
}

fn meta_operator<'s, Input>() -> impl Parser<Input, Output = CommandList>
where
    Input: Stream<Token = char>,
{
    let glue_content = meta_operator_glue().map(|c| vec![c]);
    let any_content = many1(meta_operator_item());
    let no_content = produce(|| vec![vec![OwnedFormatterCommand::ResetFormatting].into()]);

    let content = glue_content.or(any_content).or(no_content);

    between(char('{'), char('}'), content).map(|command_lists: Vec<CommandList>| {
        command_lists
            .into_iter()
            .map(|c| c.0)
            .flatten()
            .collect::<Vec<_>>()
            .into()
    })
}

fn translation<'c, Input>() -> impl Parser<Input, Output = CommandList>
where
    Input: Stream<Token = char>,
{
    let content = many(choice((
        translation_text().map(|c| vec![c].into()),
        meta_operator(),
    )))
    .map(|command_lists: Vec<CommandList>| {
        command_lists
            .into_iter()
            .map(|c| c.0)
            .flatten()
            .collect::<Vec<_>>()
            .into()
    });

    between(char('"'), lex(char('"')), content).expected("translation")
}

fn entry<Input>() -> impl Parser<Input, Output = (Outline, CommandList)>
where
    Input: Stream<Token = char>,
{
    (outline(), lex(char(':')), translation()).map(|t| (t.0, t.2))
}

/// Parses a Plover JSON dictionary. Returns an iterator over the outlines and their translations.
pub fn dict<Input>(
    input: Input,
) -> Result<impl Iterator<Item = Result<(Outline, CommandList), Input::Error>>, Input::Error>
where
    Input: Stream<Token = char>,
{
    // Parse just the header
    let mut header = lex(char('{')).expected("opening delimiter");
    let (_, content) = header.parse(input)?;

    // Carry on with the remaining content, parsing lines until we hit the last one (no comma, closing bracket instead)
    let inline_entry = optional(lex(char(',')).expected("entry delimiter")).then(move |_| entry());
    let footer = lex(char('}')).expected("closing delimiter").and(eof());
    let mut line = inline_entry.map(Some).or(footer.map(|_| None));
    let mut state = Some(content);

    Ok(core::iter::from_fn(move || {
        match state.take() {
            // If there is something left to consume, try parsing the next line.
            // In case we hit the footer, immediately return None to terminate the iterator.
            Some(remainder) => match line.parse(remainder) {
                Ok((entry, new_remainder)) => {
                    state = Some(new_remainder);
                    entry.map(Ok)
                }
                Err(error) => Some(Err(error)),
            },

            // If the previous iteration consumed the content because
            // an error was thrown, we terminate the iterator.
            None => None,
        }
    }))
}
