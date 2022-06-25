//! Tools to parse and compile data used by the operational engine like dictionaries and grammar rules

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
        for entry in output {
            let (outline, translation) = entry.unwrap();
            entries.push((outline, translation));
        }

        // 3. Create a tree data structure and allocate a buffer
        let tree = TreeNode::new(entries);
        let mut buffer = Vec::new();

        // 4. Serialize the translations into the buffer and build a list of pointers
        let translations =
            TranslationPointers::new_by_serializing_into(&mut buffer, tree.command_lists());

        // 5. Inject the starting location of the tree into the buffer
        // 		+4 because the u32 we insert four bytes at the beginning
        let tree_offset_bytes = (buffer.len() as u32 + 4).to_be_bytes();
        buffer.insert(0, tree_offset_bytes[0]);
        buffer.insert(1, tree_offset_bytes[1]);
        buffer.insert(2, tree_offset_bytes[2]);
        buffer.insert(3, tree_offset_bytes[3]);

        // 6. Serialize the tree into the buffer
        tree.serialize_into_buffer(&mut buffer, &translations);

        (tree, buffer)
    }
}
