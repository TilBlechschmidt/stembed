use core::future::Future;

use crate::formatter::{AttachmentMode, CapitalizationMode, FormatterCommand};
use crate::Stroke;
use crate::{NODE_HEADER_SIZE, PREFIX_ARRAY_SIZE_LIMIT, TRANSLATION_SIZE_LIMIT};

pub trait DataSource {
    type Error;
    type ReadFut<'s>: Future<Output = Result<(), Self::Error>> + 's
    where
        Self: 's;

    fn read_exact<'s>(&'s mut self, location: u32, buffer: &'s mut [u8]) -> Self::ReadFut<'s>;
}

pub struct RadixTreeDictionary<D: DataSource> {
    tree_start: u32,
    source: D,
}

impl<D: DataSource> RadixTreeDictionary<D> {
    pub async fn new(mut source: D) -> Result<Self, D::Error> {
        let mut tree_start_bytes = [0; 4];
        source.read_exact(0, &mut tree_start_bytes).await?;

        // TODO Somehow verify that we are looking at a valid dictionary, magic number(s) at the start?

        Ok(Self {
            tree_start: u32::from_be_bytes(tree_start_bytes),
            source,
        })
    }

    /// Loads the node header and prefix array into the `read_buffer`.
    /// Note that it is possible that too many bytes are read as the buffer will be filled completely each time.
    async fn read_node_at(&mut self, location: u32) -> Result<Node, D::Error> {
        let mut buffer = [0; NODE_HEADER_SIZE + PREFIX_ARRAY_SIZE_LIMIT];
        self.source.read_exact(location, &mut buffer).await?;
        Ok(Node::from((location, buffer)))
    }

    /// Reads the node's pointer to the given child
    async fn read_child_pointer(
        &mut self,
        node: Node,
        child_index: usize,
    ) -> Result<ChildPointer, D::Error> {
        assert!(child_index < node.child_count());

        let mut buffer = [0; 3];
        let location = node.pointer_array_location() + (child_index as u32) * 3;
        self.source.read_exact(location, &mut buffer).await?;

        let child_pointer = u32::from_be_bytes([0, buffer[0], buffer[1], buffer[2]]);

        if child_pointer >= self.tree_start {
            Ok(ChildPointer::Node(child_pointer))
        } else {
            Ok(ChildPointer::Translation(child_pointer))
        }
    }

    /// Traverses the tree finding the pointer to the translation of the entry which matches the longest prefix of the input bytes
    async fn find_longest_matching_prefix_pointer(
        &mut self,
        bytes: impl Iterator<Item = u8> + Clone,
    ) -> Result<Option<Match>, D::Error> {
        // Make the iterator peekable
        let mut bytes = bytes.peekable();

        // Offset of current tree node
        let mut location = self.tree_start;

        // Latest matching translation we encountered
        let mut latest_match: Option<Match> = None;

        // Number of bytes we have already "consumed"
        let mut prefix_len = 0;

        // Descend down the tree until we have no more bytes left or we encounter a dead end
        loop {
            // Read the node
            let node = self.read_node_at(location).await?;

            // Record the translation if there is one in case we have to backtrack
            if let Some(translation_pointer) = node.translation_pointer() {
                latest_match = Some(Match {
                    translation_pointer,
                    prefix_len,
                });
            }

            // Bail if we do not have anything left to match
            if bytes.peek().is_none() {
                break;
            }

            // If there is a matching child, descend, otherwise stop searching and use whatever we got
            match node.find_child(bytes.clone()) {
                Some(child_index) => {
                    // As we descend down the tree, increase the consumed prefix length
                    prefix_len += node.prefix_length();

                    // Since we gave find_child a clone of the iterator, we have to advance our version of it
                    for _ in 0..node.prefix_length() {
                        bytes.next();
                    }

                    // Read and check what kind of child node we have
                    match self.read_child_pointer(node, child_index).await? {
                        ChildPointer::Node(child_node_pointer) => {
                            // Descend further down the tree
                            location = child_node_pointer;
                            continue;
                        }
                        ChildPointer::Translation(translation_pointer) => {
                            // Upon encountering a leaf-only node, store it and stop searching
                            latest_match = Some(Match {
                                translation_pointer,
                                prefix_len,
                            });
                            break;
                        }
                    }
                }
                None => break,
            }
        }

        // Verify that the translation is on a stroke boundary. If it is not, then the data structure is corrupted.
        if let Some(translation) = &latest_match {
            debug_assert_eq!(translation.prefix_len % 3, 0, "encountered translation at node whose cumulative prefix length is not on a stroke boundary");
        }

        Ok(latest_match)
    }

    async fn read_translation(&mut self, location: u32) -> Result<TranslationBuffer, D::Error> {
        debug_assert!(location < self.tree_start);

        let mut buffer = [0; TRANSLATION_SIZE_LIMIT];
        self.source.read_exact(location, &mut buffer).await?;

        Ok(TranslationBuffer(buffer))
    }

    pub async fn match_prefix<'s>(
        &mut self,
        strokes: impl Iterator<Item = &'s Stroke> + Clone,
    ) -> Result<Option<(usize, TranslationBuffer)>, D::Error> {
        let stroke_bytes = strokes.cloned().flat_map(Stroke::into_bytes);

        let matched = match self
            .find_longest_matching_prefix_pointer(stroke_bytes)
            .await?
        {
            Some(matched) => {
                let translation = self.read_translation(matched.translation_pointer).await?;
                Some((matched.prefix_len / 3, translation))
            }
            None => None,
        };

        Ok(matched)
    }
}

pub struct TranslationBuffer([u8; TRANSLATION_SIZE_LIMIT]);

impl TranslationBuffer {
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    pub fn iter(&self) -> impl Iterator<Item = FormatterCommand<&str>> {
        let mut position = 0;

        core::iter::from_fn(move || match self.read_command(position) {
            Some((command, length)) => {
                position += length;
                Some(command)
            }
            None => None,
        })
    }

    /// Attempts to read a command from the given offset and returns the command as well as the number of bytes consumed.
    /// Returns `None` if a "end of command list" token is encountered.
    fn read_command(&self, offset: usize) -> Option<(FormatterCommand<&str>, usize)> {
        use AttachmentMode::*;
        use CapitalizationMode::*;
        use FormatterCommand::*;

        let buf = &self.0[offset..];

        if buf[0] & 0b11_000000 == 0 {
            let len = buf[0] as usize;
            let string_data = &buf[1..1 + len];

            // TODO Propagate the error
            let string = core::str::from_utf8(string_data)
                .expect("encountered invalid UTF8 string data in translation");

            Some((Write(string), 1 + len))
        } else {
            let command = match buf[0] {
                0b01_000_000 => ChangeCapitalization(Unchanged),
                0b01_001_000 => ChangeCapitalization(Lowercase),
                0b01_010_000 => ChangeCapitalization(Capitalize),
                0b01_011_000 => ChangeCapitalization(Uppercase),
                0b01_100_000 => ChangeCapitalization(LowerThenCapitalize),
                0b01_101_000 => ChangeCapitalization(LowercaseNext),
                0b01_110_000 => ChangeCapitalization(CapitalizeNext),
                0b01_111_000 => ChangeCapitalization(UppercaseNext),

                0b10_00_0000 => ChangeAttachment(Delimited),
                0b10_01_0000 => ChangeAttachment(Glue),
                0b10_10_0000 => ChangeAttachment(Next),
                0b10_11_0000 => ChangeAttachment(Always),

                0b110_00000 => ResetFormatting,

                0xFF => return None,

                _ => panic!("unexpected FormatterCommand byte"),
            };

            Some((command, 1))
        }
    }
}

struct Match {
    /// Location of the translation for the matched prefix
    translation_pointer: u32,

    /// Length of matched prefix in bytes
    prefix_len: usize,
}

enum ChildPointer {
    Node(u32),
    Translation(u32),
}

struct Node {
    location: u32,
    buffer: [u8; PREFIX_ARRAY_SIZE_LIMIT + NODE_HEADER_SIZE],
}

impl Node {
    fn pointer_array_location(&self) -> u32 {
        self.location + (NODE_HEADER_SIZE + self.prefix_length() * self.child_count()) as u32
    }

    /// Number of child nodes
    fn child_count(&self) -> usize {
        self.buffer[0] as usize + 1
    }

    /// Length of the prefix consumed by this node
    fn prefix_length(&self) -> usize {
        self.buffer[1] as usize
    }

    /// Location of the translation for this node
    fn translation_pointer(&self) -> Option<u32> {
        let pointer = u32::from_be_bytes([0, self.buffer[2], self.buffer[3], self.buffer[4]]);

        if pointer == 0 {
            None
        } else {
            Some(pointer)
        }
    }

    /// Slice indicating the prefix array
    fn prefix_array(&self) -> &[u8] {
        let array_len = self.prefix_length() * self.child_count();
        &self.buffer[NODE_HEADER_SIZE..][..array_len]
    }

    /// Locates a child matching the given prefix. Returns the index of the child matching the given prefix.
    /// Truncates the prefix if it is longer than the nodes prefix length and returns `None` if it is shorter.
    fn find_child(&self, prefix: impl Iterator<Item = u8> + Clone) -> Option<usize> {
        let prefix = prefix.take(self.prefix_length());

        self.prefix_array()
            .chunks_exact(self.prefix_length())
            .enumerate()
            .find_map(|(i, p)| {
                if prefix.clone().eq(p.into_iter().cloned()) {
                    Some(i)
                } else {
                    None
                }
            })
    }
}

impl From<(u32, [u8; PREFIX_ARRAY_SIZE_LIMIT + NODE_HEADER_SIZE])> for Node {
    fn from((location, buffer): (u32, [u8; PREFIX_ARRAY_SIZE_LIMIT + NODE_HEADER_SIZE])) -> Self {
        Node { location, buffer }
    }
}
