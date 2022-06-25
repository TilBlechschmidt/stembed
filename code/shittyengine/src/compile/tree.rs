//! Data structures and functions to build a tree from key-value dictionary entries

use super::json::{CommandList, Outline};
use super::TranslationPointers;
use crate::PREFIX_ARRAY_SIZE_LIMIT;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::{vec, vec::Vec};

/// Node in a radix tree containing its children and optionally some leaf data
#[derive(Debug)]
pub struct TreeNode {
    pub children: Vec<(Vec<u8>, Child)>,
    pub leaf_data: Option<CommandList>,
    pub prefix_length: usize,
}

/// Sub-node of a [`TreeNode`](TreeNode) which can either be leaf or a sub-tree node
#[derive(Debug)]
pub enum Child {
    Leaf(CommandList),
    Tree(TreeNode),
}

impl TreeNode {
    /// Recursively constructs a radix tree from the given dictionary entries
    pub fn new(entries: Vec<(Outline, CommandList)>) -> Self {
        let raw_entries = entries
            .into_iter()
            .map(|(o, t)| (o.into_bytes().collect(), t))
            .collect();

        build_tree(raw_entries)
    }

    /// Collects references to all translations stored in the tree
    pub fn command_lists(&self) -> Vec<&CommandList> {
        let mut lists = Vec::new();
        self.recursive_command_lists(&mut lists);
        lists
    }

    fn recursive_command_lists<'s>(&'s self, lists: &mut Vec<&'s CommandList>) {
        if let Some(data) = self.leaf_data.as_ref() {
            lists.push(data);
        }

        for (_, child) in self.children.iter() {
            match child {
                Child::Leaf(data) => lists.push(&data),
                Child::Tree(node) => node.recursive_command_lists(lists),
            }
        }
    }

    fn into_child(self) -> Child {
        if self.children.is_empty() {
            let leaf_data = self
                .leaf_data
                .expect("encountered node with no children and no leaf data");
            Child::Leaf(leaf_data)
        } else {
            Child::Tree(self)
        }
    }

    // Note that this function expects the buffer to already contain the translation data at the beginning
    pub fn serialize_into_buffer(&self, buffer: &mut Vec<u8>, translations: &TranslationPointers) {
        assert!(
            self.children.len() < u8::MAX as usize,
            "node contains more than 255 children"
        );
        assert!(
            !self.children.is_empty(),
            "encountered node without children"
        );
        assert!(
            self.prefix_length < u8::MAX as usize,
            "encountered node with prefix length larger than 255"
        );

        // Store the number of children, offset by one as empty nodes are disallowed
        buffer.push((self.children.len() - 1) as u8);

        // Store the prefix length
        buffer.push(self.prefix_length as u8);

        // Store information about the translation location if applicable
        if let Some(leaf_data) = &self.leaf_data {
            let offset: u32 = translations
                .offset_for(leaf_data)
                .expect("encountered leaf data not available in translations storage")
                .try_into()
                .unwrap();

            let bytes = offset.to_be_bytes();
            assert_eq!(
                bytes[0], 0,
                "encountered translation pointer that is out-of-range"
            );
            buffer.extend(&bytes[1..]);
        } else {
            // In case there is no translation, store zeros.
            // TODO Make this less of a magic number
            buffer.extend([0, 0, 0]);
        }

        // Build the prefix/key array
        for (prefix, _) in &self.children {
            // TODO This fails and it is unacceptable for this to fail.
            //      The reason is probably that a node contains mostly children with length X
            //      but there is that one child which only has Y bytes remaining where Y < X.
            assert_eq!(
                prefix.len(),
                self.prefix_length,
                "encountered child with mismatching prefix length"
            );
            buffer.extend(prefix);
        }

        // Allocate the pointer array
        let pointer_array_start = buffer.len();
        buffer.extend((0..self.children.len() * 3).map(|_| 0));

        // Serialize the child nodes and store pointers to them
        for (i, (_, child)) in self.children.iter().enumerate() {
            match child {
                Child::Leaf(data) => {
                    let offset: u32 = translations
                        .offset_for(&data)
                        .expect("encountered leaf data not available in translations storage")
                        .try_into()
                        .unwrap();

                    let data_location = offset.to_be_bytes();
                    assert_eq!(
                        data_location[0], 0,
                        "encountered translation pointer that is out-of-range"
                    );
                    buffer[pointer_array_start + i * 3 + 0] = data_location[1];
                    buffer[pointer_array_start + i * 3 + 1] = data_location[2];
                    buffer[pointer_array_start + i * 3 + 2] = data_location[3];
                }
                Child::Tree(node) => {
                    let child_location = buffer.len().to_be_bytes();
                    buffer[pointer_array_start + i * 3 + 0] = child_location[1];
                    buffer[pointer_array_start + i * 3 + 1] = child_location[2];
                    buffer[pointer_array_start + i * 3 + 2] = child_location[3];

                    node.serialize_into_buffer(buffer, translations);
                }
            }
        }
    }
}

fn build_tree(entries: Vec<(Vec<u8>, CommandList)>) -> TreeNode {
    let prefix_length = calculate_prefix_length(&entries);
    let mut prefix_map = BTreeMap::<Vec<u8>, Vec<(Vec<u8>, CommandList)>>::new();
    let mut leaf_data = None;

    // Group the entries by prefix
    for (key, value) in entries.into_iter() {
        if key.is_empty() {
            assert!(
                leaf_data.is_none(),
                "encountered multiple translations for the same stroke"
            );
            leaf_data = Some(value);
        } else {
            let (prefix_ref, remainder_ref) = key.split_at(prefix_length.min(key.len()));
            let prefix = Vec::from(prefix_ref);
            let remainder = Vec::from(remainder_ref);

            prefix_map
                .entry(prefix)
                .and_modify(|e| e.push((remainder.clone(), value.clone())))
                .or_insert(vec![(remainder, value)]);
        }
    }

    // Build child nodes from the groups
    let children = prefix_map
        .into_iter()
        .map(|(prefix, entries)| {
            let child = build_tree(entries).into_child();
            (prefix, child)
        })
        .collect();

    // Build the node from parts
    TreeNode {
        children,
        leaf_data,
        prefix_length,
    }
}

fn calculate_prefix_length(entries: &Vec<(Vec<u8>, CommandList)>) -> usize {
    let mut prefix_length = 1;
    let max_prefix_length = entries
        .iter()
        .map(|(bytes, _)| bytes.len())
        .min()
        .unwrap_or(1);

    if max_prefix_length == 0 {
        return max_prefix_length;
    }

    for current_prefix_length in 1..max_prefix_length {
        let prefixes = entries
            .iter()
            .map(|(bytes, _)| bytes.iter().take(current_prefix_length).cloned().collect());

        let mut unique_prefixes = BTreeSet::<Vec<u8>>::new();
        unique_prefixes.extend(prefixes);

        if unique_prefixes.is_empty()
            || unique_prefixes.len() * current_prefix_length > PREFIX_ARRAY_SIZE_LIMIT
        {
            break;
        } else {
            prefix_length = current_prefix_length;
        }
    }

    prefix_length
}
