//! Data structures to construct a deduplicated binary blob containing all translations

use super::json::CommandList;
use crate::formatter::{AttachmentMode, CapitalizationMode, FormatterCommand};
use crate::TRANSLATION_SIZE_LIMIT;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::fmt::Display;

pub struct TranslationPointers<'a> {
    pointers: BTreeMap<&'a CommandList, usize>,
}

impl<'a> TranslationPointers<'a> {
    pub fn new_by_serializing_into(buffer: &mut Vec<u8>, entries: Vec<&'a CommandList>) -> Self {
        let mut pointers = BTreeMap::new();

        for entry in entries {
            if pointers.contains_key(entry) {
                continue;
            }

            let data = entry.to_bytes();
            let location = buffer.len();

            pointers.insert(entry, location);
            buffer.extend(data);
        }

        Self { pointers }
    }

    pub fn offset_for(&self, entry: &'a CommandList) -> Option<usize> {
        self.pointers.get(entry).cloned()
    }
}

impl CommandList {
    fn to_bytes(&self) -> Vec<u8> {
        let mut data = Vec::new();

        for entry in self.iter() {
            data.extend(entry.to_bytes());
        }

        data.push(0xFF);

        assert!(data.len() < TRANSLATION_SIZE_LIMIT);

        data
    }
}

impl<S: AsRef<str> + Display> FormatterCommand<S> {
    fn to_bytes(&self) -> Vec<u8> {
        use AttachmentMode::*;
        use CapitalizationMode::*;
        use FormatterCommand::*;

        let mut string_data = None;
        let mut data = Vec::<u8>::new();

        data.push(match self {
            Write(string) => {
                let string = string.as_ref();
                assert!(
                    string.len() < 2usize.pow(6),
                    "strings longer than 63 characters are currently not supported (processing '{string}')"
                );
                string_data = Some(string);
                0b00_000000 | (string.len() as u8)
            }

            ChangeCapitalization(Unchanged) => 0b01_000_000,
            ChangeCapitalization(Lowercase) => 0b01_001_000,
            ChangeCapitalization(Capitalize) => 0b01_010_000,
            ChangeCapitalization(Uppercase) => 0b01_011_000,
            ChangeCapitalization(LowerThenCapitalize) => 0b01_100_000,
            ChangeCapitalization(LowercaseNext) => 0b01_101_000,
            ChangeCapitalization(CapitalizeNext) => 0b01_110_000,
            ChangeCapitalization(UppercaseNext) => 0b01_111_000,

            ChangeAttachment(Delimited) => 0b10_00_0000,
            ChangeAttachment(Glue) => 0b10_01_0000,
            ChangeAttachment(Next) => 0b10_10_0000,
            ChangeAttachment(Always) => 0b10_11_0000,

            ResetFormatting => 0b110_00000,
        });

        if let Some(string_data) = string_data {
            data.extend(string_data.bytes());
        }

        data
    }
}
