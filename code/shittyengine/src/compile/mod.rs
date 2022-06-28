//! Tools to parse and compile data used by the operational engine like dictionaries and grammar rules

use crate::{
    dict::{DataSource, RadixTreeDictionary},
    formatter::FormatterCommand,
};
use alloc::{string::String, vec, vec::Vec};

mod json;

mod tree;
pub use tree::*;

mod translations;
use translations::TranslationPointers;

pub struct Compiler;

impl Compiler {
    /// Builds a compiled tree from a JSON dictionary. Returns the raw tree and the compiled version.
    /// Panics if the input data is invalid.
    // TODO Propagate the errors instead of panicking
    pub fn compile_from_json(json: &str) -> (TreeNode, Vec<u8>) {
        // 1. Parse the dictionary
        let output = json::dict(json).unwrap();

        // 2. Build an array of entries
        let mut entries = Vec::new();
        let mut longest_outline_len = 0;
        for entry in output {
            let (outline, translation) = entry.unwrap();
            longest_outline_len = longest_outline_len.max(outline.len());
            entries.push((outline, translation));
        }

        // 3. Create a tree data structure and allocate a buffer
        let tree = TreeNode::new(entries.clone());
        let mut buffer = vec![0, 0, 0, 0];

        // 4. Serialize the translations into the buffer and build a list of pointers
        let translations =
            TranslationPointers::new_by_serializing_into(&mut buffer, tree.command_lists());

        // 5. Inject the starting location of the tree into the buffer
        let tree_offset_bytes = (buffer.len() as u32).to_be_bytes();
        buffer[0] = tree_offset_bytes[0];
        buffer[1] = tree_offset_bytes[1];
        buffer[2] = tree_offset_bytes[2];
        buffer[3] = tree_offset_bytes[3];

        // 6. Serialize the tree into the buffer
        tree.serialize_into_buffer(&mut buffer, &translations);

        // 7. Verify that all the entries are readable and return the correct translation
        let mut source = BufferedSource::new(&buffer);
        let mut dict = RadixTreeDictionary::new(&mut source)
            .expect("failed to construct dictionary from compiled buffer");

        for (outline, commands) in entries {
            let outline_len = outline.len();
            let match_err = alloc::format!("outline {outline} not found in compiled dictionary");

            let (stroke_count, translation) = dict
                .match_prefix(outline.iter())
                .expect("unexpected buffer read error while verifying dictionary")
                .expect(&match_err);

            let translation_vec = translation.iter().collect::<Vec<_>>();
            assert_eq!(outline_len, stroke_count);
            assert_eq!(commands.0, translation_vec);
        }

        (tree, buffer)
    }
}

/// In-memory data source for RadixTreeDictionary
pub struct BufferedSource<'b> {
    buffer: &'b Vec<u8>,
}

impl<'b> BufferedSource<'b> {
    pub fn new(buffer: &'b Vec<u8>) -> Self {
        Self { buffer }
    }
}

impl<'b> DataSource for &mut BufferedSource<'b> {
    type Error = ();

    fn read_exact(&mut self, location: u32, buffer: &mut [u8]) -> Result<(), Self::Error> {
        if location as usize + buffer.len() > self.buffer.len() {
            let slice = &self.buffer[location as usize..];
            buffer[0..slice.len()].copy_from_slice(slice);
        } else {
            let range = location as usize..(location as usize + buffer.len());
            buffer.copy_from_slice(&self.buffer[range]);
        }

        Ok(())
    }
}

// Helper implementation for verifying fetched commands against parsed commands
impl PartialEq<FormatterCommand<&str>> for FormatterCommand<String> {
    fn eq(&self, other: &FormatterCommand<&str>) -> bool {
        use FormatterCommand::*;

        let self_ref: FormatterCommand<&str> = match self {
            Write(string) => Write(string.as_ref()),
            ChangeCapitalization(c) => ChangeCapitalization(*c),
            ChangeAttachment(a) => ChangeAttachment(*a),
            ResetFormatting => ResetFormatting,
        };

        self_ref.eq(other)
    }
}
