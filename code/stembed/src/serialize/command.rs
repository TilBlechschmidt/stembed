use crate::{
    core::{
        engine::{Command, EngineCommand},
        processor::text_formatter::{AttachmentMode, CapitalizationMode, TextOutputCommand},
    },
    serialize::{Deserialize, Serialize},
};
use alloc::{string::String, vec::Vec};

// The first bit encodes the variant of the `Command` enum
const COMMAND_VARIANT_MASK: u8 = 0b10000000;
const COMMAND_VARIANT_ENGINE: u8 = 0b00000000;
const COMMAND_VARIANT_OUTPUT: u8 = 0b10000000;

const OUTPUT_VARIANT_MASK: u8 = 0b01110000;
const OUTPUT_VARIANT_TEXT: u8 = 0b00000000;
const OUTPUT_VARIANT_DELIMITER: u8 = 0b00010000;
const OUTPUT_VARIANT_CAPITALIZATION: u8 = 0b00100000;
const OUTPUT_VARIANT_ATTACHMENT: u8 = 0b00110000;
const OUTPUT_VARIANT_RESET: u8 = 0b01110000;

const CAPITALIZATION_VARIANT_MASK: u8 = 0b00001110;
const CAPITALIZATION_VARIANT_NONE: u8 = 0b00000000;
const CAPITALIZATION_VARIANT_LOWER: u8 = 0b00000010;
const CAPITALIZATION_VARIANT_CAPIT: u8 = 0b00000100;
const CAPITALIZATION_VARIANT_UPPER: u8 = 0b00000110;
const CAPITALIZATION_VARIANT_LOWER_THEN_CAPIT: u8 = 0b00001000;
const CAPITALIZATION_VARIANT_LOWER_NEXT: u8 = 0b00001010;
const CAPITALIZATION_VARIANT_CAPIT_NEXT: u8 = 0b00001100;
const CAPITALIZATION_VARIANT_UPPER_NEXT: u8 = 0b00001110;

const ATTACHMENT_VARIANT_MASK: u8 = 0b00001100;
const ATTACHMENT_VARIANT_DELIMITED: u8 = 0b00000000;
const ATTACHMENT_VARIANT_GLUE: u8 = 0b00000100;
const ATTACHMENT_VARIANT_NEXT: u8 = 0b00001000;
const ATTACHMENT_VARIANT_ALWAYS: u8 = 0b00001100;

const TEXT_OUTPUT_LENGTH_MASK: u8 = 0b00001111;

impl Serialize for Command<TextOutputCommand> {
    type Error = crate::io::Error;

    fn serialize(&self, writer: &mut impl crate::io::Write) -> Result<(), Self::Error> {
        use AttachmentMode::*;
        use CapitalizationMode::*;

        match self {
            Command::Engine(_) => writer.write_u8(COMMAND_VARIANT_ENGINE),
            Command::Output(command) => match command {
                TextOutputCommand::Write(string) => {
                    // TODO Implement proper error handling
                    assert!(string.len() < 4096 /* 12-bit length */);

                    let length = string.len() as u16;
                    let length_lower = (length & 0b11111111) as u8;
                    let length_upper = ((length & 0b111100000000) >> 8) as u8;

                    writer.write_u8(COMMAND_VARIANT_OUTPUT | OUTPUT_VARIANT_TEXT | length_upper)?;
                    writer.write_u8(length_lower)?;

                    for byte in string.as_bytes().into_iter() {
                        writer.write_u8(*byte)?;
                    }

                    Ok(())
                }
                TextOutputCommand::ChangeDelimiter(delimiter) => {
                    writer.write_u8(COMMAND_VARIANT_OUTPUT | OUTPUT_VARIANT_DELIMITER)?;
                    writer.write_u32(*delimiter as u32)
                }
                TextOutputCommand::ChangeCapitalization(capitalization_mode) => {
                    let mode_bits = match capitalization_mode {
                        None => CAPITALIZATION_VARIANT_NONE,
                        Uppercase => CAPITALIZATION_VARIANT_UPPER,
                        Lowercase => CAPITALIZATION_VARIANT_LOWER,
                        Capitalize => CAPITALIZATION_VARIANT_CAPIT,
                        LowerThenCapitalize => CAPITALIZATION_VARIANT_LOWER_THEN_CAPIT,
                        UppercaseNext => CAPITALIZATION_VARIANT_UPPER_NEXT,
                        LowercaseNext => CAPITALIZATION_VARIANT_LOWER_NEXT,
                        CapitalizeNext => CAPITALIZATION_VARIANT_CAPIT_NEXT,
                    };

                    writer.write_u8(
                        COMMAND_VARIANT_OUTPUT | OUTPUT_VARIANT_CAPITALIZATION | mode_bits,
                    )
                }
                TextOutputCommand::ChangeAttachment(attachment_mode) => {
                    let mode_bits = match attachment_mode {
                        Delimited => ATTACHMENT_VARIANT_DELIMITED,
                        Glue => ATTACHMENT_VARIANT_GLUE,
                        Next => ATTACHMENT_VARIANT_NEXT,
                        Always => ATTACHMENT_VARIANT_ALWAYS,
                    };

                    writer.write_u8(COMMAND_VARIANT_OUTPUT | OUTPUT_VARIANT_ATTACHMENT | mode_bits)
                }
                TextOutputCommand::ResetFormatting => {
                    writer.write_u8(COMMAND_VARIANT_OUTPUT | OUTPUT_VARIANT_RESET)
                }
            },
        }
    }
}

impl Deserialize for Command<TextOutputCommand> {
    type Context = ();
    type Error = crate::io::Error;

    fn deserialize(
        reader: &mut impl crate::io::Read,
        _context: &Self::Context,
    ) -> Result<Self, Self::Error> {
        use AttachmentMode::*;
        use CapitalizationMode::*;

        let data = reader.read_u8()?;

        match data & COMMAND_VARIANT_MASK {
            COMMAND_VARIANT_ENGINE => Ok(Command::Engine(EngineCommand::UndoPrevious)),
            COMMAND_VARIANT_OUTPUT => {
                let output_command = match data & OUTPUT_VARIANT_MASK {
                    OUTPUT_VARIANT_TEXT => {
                        let mut length: u16 = ((data & TEXT_OUTPUT_LENGTH_MASK) as u16) << 8;
                        length |= reader.read_u8()? as u16;

                        // Strings allocate anyways so we can just use Vec
                        let mut data = Vec::with_capacity(length as usize);

                        for _ in 0..length {
                            data.push(reader.read_u8()?);
                        }

                        // TODO Implement proper error handling/propagation when encountering invalid UTF-8 data
                        TextOutputCommand::Write(
                            String::from_utf8(data).expect("encountered invalid UTF-8 data"),
                        )
                    }
                    OUTPUT_VARIANT_DELIMITER => {
                        let delimiter_data = reader.read_u32()?;
                        // TODO Implement proper error handling/propagation when encountering invalid UTF-8 data
                        let delimiter =
                            char::from_u32(delimiter_data).expect("encountered invalid UTF-8 data");
                        TextOutputCommand::ChangeDelimiter(delimiter)
                    }
                    OUTPUT_VARIANT_CAPITALIZATION => {
                        let capitalization_mode = match data & CAPITALIZATION_VARIANT_MASK {
                            CAPITALIZATION_VARIANT_NONE => None,
                            CAPITALIZATION_VARIANT_LOWER => Lowercase,
                            CAPITALIZATION_VARIANT_CAPIT => Capitalize,
                            CAPITALIZATION_VARIANT_UPPER => Uppercase,
                            CAPITALIZATION_VARIANT_LOWER_THEN_CAPIT => LowerThenCapitalize,
                            CAPITALIZATION_VARIANT_LOWER_NEXT => LowercaseNext,
                            CAPITALIZATION_VARIANT_CAPIT_NEXT => CapitalizeNext,
                            CAPITALIZATION_VARIANT_UPPER_NEXT => UppercaseNext,
                            _ => unreachable!(),
                        };

                        TextOutputCommand::ChangeCapitalization(capitalization_mode)
                    }
                    OUTPUT_VARIANT_ATTACHMENT => {
                        let attachment_mode = match data & ATTACHMENT_VARIANT_MASK {
                            ATTACHMENT_VARIANT_DELIMITED => Delimited,
                            ATTACHMENT_VARIANT_GLUE => Glue,
                            ATTACHMENT_VARIANT_NEXT => Next,
                            ATTACHMENT_VARIANT_ALWAYS => Always,
                            _ => unreachable!(),
                        };

                        TextOutputCommand::ChangeAttachment(attachment_mode)
                    }
                    OUTPUT_VARIANT_RESET => TextOutputCommand::ResetFormatting,
                    _ => unreachable!(),
                };

                Ok(Command::Output(output_command))
            }
            _ => unreachable!(),
        }
    }
}
