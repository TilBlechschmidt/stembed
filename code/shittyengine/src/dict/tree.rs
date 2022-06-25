use crate::{NODE_HEADER_SIZE, PREFIX_ARRAY_SIZE_LIMIT, TRANSLATION_SIZE_LIMIT};

const _: () = assert!(TRANSLATION_SIZE_LIMIT <= PREFIX_ARRAY_SIZE_LIMIT, "RadixTreeDictionary requires maximum translation size to be equal to or less than the maximum prefix array size as they share the same buffer");

pub trait DataSource {
    type Error;
    fn read_exact(&mut self, location: u32, buffer: &mut [u8]) -> Result<(), Self::Error>;
}

pub struct RadixTreeDictionary<D: DataSource> {
    /// Dual-use buffer for reading the prefix arrays while traversing the tree and temporarily storing translation data once we found a matching entry
    buffer: [u8; PREFIX_ARRAY_SIZE_LIMIT + NODE_HEADER_SIZE],
    tree_start: u32,
    source: D,
}

impl<D: DataSource> RadixTreeDictionary<D> {
    pub fn new(mut source: D) -> Result<Self, D::Error> {
        let mut tree_start_bytes = [0; 4];
        source.read_exact(0, &mut tree_start_bytes)?;

        // TODO Somehow verify that we are looking at a valid dictionary, magic number(s) at the start?

        Ok(Self {
            buffer: [0; PREFIX_ARRAY_SIZE_LIMIT + NODE_HEADER_SIZE],
            tree_start: u32::from_be_bytes(tree_start_bytes),
            source,
        })
    }

    /// Loads the node header and prefix array into the `read_buffer`.
    /// Note that it is possible that too many bytes are read as the buffer will be filled completely each time.
    fn read_node_at(&mut self, location: u32) -> Result<Node<'_>, D::Error> {
        self.source.read_exact(location, &mut self.buffer)?;
        Ok(Node::from((location, &self.buffer)))
    }

    /// Reads the node's pointer to the given child
    fn read_child_pointer(
        &mut self,
        node: Node<'_>,
        child_index: usize,
    ) -> Result<ChildPointer, D::Error> {
        assert!(child_index < node.child_count());

        let mut buffer = [0; 3];
        let location = node.pointer_array_location() + (child_index as u32) * 3;
        self.source.read_exact(location, &mut buffer)?;

        let child_pointer = u32::from_be_bytes([0, buffer[0], buffer[1], buffer[2]]);

        if child_pointer >= self.tree_start {
            Ok(ChildPointer::Node(child_pointer))
        } else {
            Ok(ChildPointer::Translation(child_pointer))
        }
    }
}

enum ChildPointer {
    Node(u32),
    Translation(u32),
}

struct Node<'b> {
    location: u32,
    buffer: &'b [u8; PREFIX_ARRAY_SIZE_LIMIT + NODE_HEADER_SIZE],
}

impl<'b> Node<'b> {
    fn pointer_array_location(&self) -> u32 {
        self.location + (NODE_HEADER_SIZE + self.prefix_length() * self.child_count()) as u32
    }

    /// Number of child nodes
    fn child_count(&self) -> usize {
        self.buffer[0] as usize
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

    /// Locates the given prefix in this node. Returns the index of the child matching the given prefix.
    /// Truncates the prefix if it is longer than the nodes prefix length and returns `None` if it is shorter.
    fn find_prefix(&self, prefix: &[u8]) -> Option<usize> {
        let prefix_len = self.prefix_length();

        if prefix.len() < prefix_len {
            None
        } else {
            let prefix = &prefix[..prefix_len];
            self.buffer
                .chunks_exact(prefix_len)
                .enumerate()
                .find_map(|(i, p)| if p == prefix { Some(i) } else { None })
        }
    }
}

impl<'b> From<(u32, &'b [u8; PREFIX_ARRAY_SIZE_LIMIT + NODE_HEADER_SIZE])> for Node<'b> {
    fn from(
        (location, buffer): (u32, &'b [u8; PREFIX_ARRAY_SIZE_LIMIT + NODE_HEADER_SIZE]),
    ) -> Self {
        Node { location, buffer }
    }
}
