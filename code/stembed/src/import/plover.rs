use crate::{
    constants::AVG_CMD_COUNT,
    core::{
        dict::{binary::Outline, CommandList},
        engine::{Command, EngineCommand},
        processor::text_formatter::{AttachmentMode, CapitalizationMode, TextOutputCommand},
        Stroke, StrokeContext,
    },
};
use alloc::{borrow::ToOwned, string::String, vec::Vec};
use combine::{
    any, attempt, between, choice, eof,
    error::{Commit, StreamError},
    many, many1, one_of, optional,
    parser::{
        char::{char, spaces, string},
        function,
    },
    produce, satisfy_map, sep_by1, ParseError, Parser, Stream, StreamOnce,
};
use smallvec::{smallvec, SmallVec};

// TODO This function is horrendously expensive in terms of heap usage — get rid of it when migrating to parsing strokes with combine, instead use combinators :)
fn preprocess(stroke: &str) -> String {
    if !stroke.contains(char::is_numeric) {
        return stroke.to_owned();
    } else {
        // Replace numbers by correct keys, add # in front if there are numbers involved
        let prefix = if stroke.starts_with('#') { "" } else { "#" };
        let mapped_stroke = stroke
            .replace('0', "O")
            .replace('1', "S")
            .replace('2', "T")
            .replace('3', "P")
            .replace('4', "H")
            .replace('5', "A")
            .replace('6', "F")
            .replace('7', "P")
            .replace('8', "L")
            .replace('9', "T");
        format!("{prefix}{mapped_stroke}")
    }
}

/// Matches the output of `p` as long as the output does not fulfill `condition` – does not consume input on failure
fn not<Input, P, F>(mut p: P, condition: F) -> impl Parser<Input, Output = P::Output>
where
    F: Fn(&P::Output) -> bool + 'static,
    P: Parser<Input>,
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    function::parser(move |input: &mut Input| {
        let (c, committed) = p.parse_lazy(input).into_result()?;

        if !condition(&c) {
            Ok((c, committed))
        } else {
            Err(Commit::Peek(Input::Error::empty(input.position()).into()))
        }
    })
}

fn lex<Input, P>(p: P) -> impl Parser<Input, Output = P::Output>
where
    P: Parser<Input>,
    Input: Stream<Token = char>,
    <Input as StreamOnce>::Error: ParseError<
        <Input as StreamOnce>::Token,
        <Input as StreamOnce>::Range,
        <Input as StreamOnce>::Position,
    >,
{
    p.skip(spaces())
}

fn json_char<Input>() -> impl Parser<Input, Output = char>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
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

fn stroke_char<Input>() -> impl Parser<Input, Output = char>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    not(json_char(), |c| *c == '/')
}

fn stroke<'c, Input>(context: &'c StrokeContext) -> impl Parser<Input, Output = Stroke<'c>>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    many(stroke_char()).and_then(move |stroke: String| {
        Stroke::from_str(preprocess(&stroke), &context)
            .map_err(
                <Input::Error as ParseError<
                    Input::Token,
                    Input::Range,
                    Input::Position,
                >>::StreamError::message_format
            )
    })
}

fn outline<'c, Input>(context: &'c StrokeContext) -> impl Parser<Input, Output = Outline<'c>>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    between(
        char('"'),
        lex(char('"')),
        sep_by1(stroke(context), char('/')),
    )
    .expected("outline")
}

// TODO This one does basically the same as json_char, merge them into one parametric parser
fn translation_char<Input>() -> impl Parser<Input, Output = char>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
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

fn translation_text<Input>() -> impl Parser<Input, Output = TextOutputCommand>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    many1(translation_char()).map(TextOutputCommand::Write)
}

fn meta_operator_item<Input>() -> impl Parser<Input, Output = CommandList<TextOutputCommand>>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    choice((
        char('$').map(|_| smallvec![Command::Engine(EngineCommand::UndoPrevious)]),
        char('^').map(|_| {
            smallvec![Command::Output(TextOutputCommand::ChangeAttachment(
                AttachmentMode::Next
            ))]
        }),
        char('>').map(|_| {
            smallvec![Command::Output(TextOutputCommand::ChangeCapitalization(
                CapitalizationMode::LowercaseNext
            ))]
        }),
        char('<').map(|_| {
            smallvec![Command::Output(TextOutputCommand::ChangeCapitalization(
                CapitalizationMode::UppercaseNext
            ))]
        }),
        attempt(string("-|")).map(|_| {
            smallvec![Command::Output(TextOutputCommand::ChangeCapitalization(
                CapitalizationMode::CapitalizeNext
            ))]
        }),
        char(',').map(|punctuation: char| {
            smallvec![
                Command::Output(TextOutputCommand::ChangeAttachment(AttachmentMode::Next)),
                Command::Output(TextOutputCommand::Write(punctuation.into())),
            ]
        }),
        one_of(".:;!?".chars()).map(|punctuation: char| {
            smallvec![
                Command::Output(TextOutputCommand::ChangeAttachment(AttachmentMode::Next)),
                Command::Output(TextOutputCommand::Write(punctuation.into())),
                Command::Output(TextOutputCommand::ChangeCapitalization(
                    CapitalizationMode::CapitalizeNext
                ))
            ]
        }),
        translation_text()
            .map(|c| smallvec![Command::Output(c)])
            .expected("string"),
    ))
}

fn meta_operator_glue<Input>() -> impl Parser<Input, Output = CommandList<TextOutputCommand>>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    char('&').then(|_| {
        translation_text()
            .map(|c| {
                smallvec![
                    Command::Output(TextOutputCommand::ChangeAttachment(AttachmentMode::Glue)),
                    Command::Output(c),
                    Command::Output(TextOutputCommand::ChangeAttachment(AttachmentMode::Glue)),
                ]
            })
            .expected("string")
    })
}

fn meta_operator<Input>() -> impl Parser<Input, Output = CommandList<TextOutputCommand>>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let glue_content = meta_operator_glue().map(|c| smallvec![c]);
    let any_content = many1(meta_operator_item());
    let no_content = produce(|| {
        smallvec![smallvec![Command::Output(
            TextOutputCommand::ResetFormatting
        )]]
    });

    let content = glue_content.or(any_content).or(no_content);

    between(char('{'), char('}'), content).map(
        |command_lists: SmallVec<[CommandList<TextOutputCommand>; AVG_CMD_COUNT]>| {
            command_lists.into_iter().flatten().collect()
        },
    )
}

fn translation<'c, Input>() -> impl Parser<Input, Output = CommandList<TextOutputCommand>>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let content = many(choice((
        translation_text().map(|c| smallvec![Command::Output(c)]),
        meta_operator(),
    )))
    .map(|command_lists: Vec<CommandList<TextOutputCommand>>| {
        command_lists.into_iter().flatten().collect()
    });

    between(char('"'), lex(char('"')), content).expected("translation")
}

fn entry<'c, Input>(
    context: &'c StrokeContext,
) -> impl Parser<Input, Output = (Outline<'c>, CommandList<TextOutputCommand>)>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    (outline(context), lex(char(':')), translation()).map(|t| (t.0, t.2))
}

pub fn parse_dict<'c, Input>(
    input: Input,
    context: &'c StrokeContext,
) -> Result<
    impl Iterator<Item = Result<(Outline<'c>, CommandList<TextOutputCommand>), Input::Error>>,
    Input::Error,
>
where
    Input: Stream<Token = char>,
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    // Parse just the header
    let mut header = lex(char('{')).expected("opening delimiter");
    let (_, content) = header.parse(input)?;

    // Carry on with the remaining content, parsing lines until we hit the last one (no comma, closing bracket instead)
    let inline_entry =
        optional(lex(char(',')).expected("entry delimiter")).then(move |_| entry(&context));
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
