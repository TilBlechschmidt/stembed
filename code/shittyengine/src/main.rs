extern crate alloc;
use alloc::collections::BTreeMap;
use shittyengine::{
    compile::{Child, Compiler, TreeNode},
    formatter::FormatterCommand,
};
use std::collections::HashSet;
use std::fmt::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = "/Users/tibl/Library/Application Support/plover/main.json";
    let content = std::fs::read_to_string(path)?;
    let (tree, buffer) = Compiler::compile_from_json(content.as_str());
    let info = TreeInfo::new(&tree);

    println!("{info}");
    println!("total bytes: {}", buffer.len());

    std::fs::write("./dict.bin", &buffer)?;

    Ok(())
}

#[derive(Default, Debug)]
struct TreeSizeInfo {
    node_overhead: usize,
    key_array: usize,
    pointer_array: usize,
    leaf_data: usize,
}

#[derive(Default, Debug)]
struct TreeInfo {
    depth: usize,
    estimated_size: TreeSizeInfo,
    nodes: BTreeMap<usize, usize>,
    prefix_lengths: BTreeMap<usize, Vec<usize>>,
    child_counts: BTreeMap<usize, Vec<usize>>,
    leaf_child_counts: BTreeMap<usize, Vec<usize>>,
    cumulative_leaf_nodes: BTreeMap<usize, usize>,

    seen_translations: HashSet<Vec<FormatterCommand<String>>>,
}

impl TreeInfo {
    pub fn new(node: &TreeNode) -> Self {
        let mut info = TreeInfo::default();
        info.read_layer(node, 0);
        info
    }

    pub fn avg_prefix_length(&self) -> BTreeMap<usize, f64> {
        self.prefix_lengths
            .iter()
            .map(|(k, v)| {
                let len = v.len();
                (*k, v.into_iter().sum::<usize>() as f64 / len as f64)
            })
            .collect()
    }

    pub fn max_prefix_length(&self) -> BTreeMap<usize, usize> {
        self.prefix_lengths
            .iter()
            .map(|(k, v)| (*k, v.into_iter().max().cloned().unwrap_or_default()))
            .collect()
    }

    pub fn avg_child_counts(&self) -> BTreeMap<usize, f64> {
        self.child_counts
            .iter()
            .map(|(k, v)| {
                let len = v.len();
                (*k, v.into_iter().sum::<usize>() as f64 / len as f64)
            })
            .collect()
    }

    pub fn max_child_counts(&self) -> BTreeMap<usize, usize> {
        self.child_counts
            .iter()
            .map(|(k, v)| (*k, v.into_iter().max().cloned().unwrap_or_default()))
            .collect()
    }

    pub fn avg_leaf_child_counts(&self) -> BTreeMap<usize, f64> {
        self.leaf_child_counts
            .iter()
            .map(|(k, v)| {
                let len = v.len();
                (*k, v.into_iter().sum::<usize>() as f64 / len as f64)
            })
            .collect()
    }

    pub fn max_leaf_child_counts(&self) -> BTreeMap<usize, usize> {
        self.leaf_child_counts
            .iter()
            .map(|(k, v)| (*k, v.into_iter().max().cloned().unwrap_or_default()))
            .collect()
    }

    pub fn avg_leaf_nodes(&self) -> BTreeMap<usize, f64> {
        self.average_value(&self.cumulative_leaf_nodes)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.values().cloned().sum::<usize>()
    }

    fn average_value(&self, input: &BTreeMap<usize, usize>) -> BTreeMap<usize, f64> {
        let mut map = BTreeMap::new();

        for (layer, value) in input.iter() {
            if let Some(node_count) = self.nodes.get(layer) {
                let average_value = *value as f64 / *node_count as f64;
                map.insert(*layer, average_value);
            }
        }

        map
    }

    fn track_leaf_data_usage(&mut self, data: &Vec<FormatterCommand<String>>) {
        if self.seen_translations.insert(data.clone()) {
            for entry in data {
                self.estimated_size.leaf_data += match entry {
                    shittyengine::formatter::FormatterCommand::Write(string) => {
                        string.as_bytes().len() + 1
                    }
                    shittyengine::formatter::FormatterCommand::ChangeCapitalization(_) => 1,
                    shittyengine::formatter::FormatterCommand::ChangeAttachment(_) => 1,
                    shittyengine::formatter::FormatterCommand::ResetFormatting => 1,
                }
            }

            self.estimated_size.leaf_data += 1;
        }
    }

    fn read_layer(&mut self, node: &TreeNode, layer: usize) {
        self.depth = self.depth.max(layer);

        // Add the node overhead
        self.estimated_size.node_overhead += 5;

        // Add the key array
        self.estimated_size.key_array += node.prefix_length * node.children.len();

        // Add the pointer array
        self.estimated_size.pointer_array += 3 * node.children.len();

        // Add the leaf data size
        if let Some(leaf_data) = &node.leaf_data {
            self.track_leaf_data_usage(leaf_data);
        }

        self.nodes.entry(layer).and_modify(|v| *v += 1).or_insert(1);

        if node.prefix_length > 0 {
            self.prefix_lengths
                .entry(layer)
                .and_modify(|v| v.push(node.prefix_length))
                .or_insert(vec![node.prefix_length]);
        }

        self.child_counts
            .entry(layer)
            .and_modify(|v| v.push(node.children.len()))
            .or_insert(vec![node.children.len()]);

        if node.leaf_data.is_some() {
            self.cumulative_leaf_nodes
                .entry(layer)
                .and_modify(|v| *v += 1)
                .or_insert(1);
        }

        let mut leaf_children = 0;
        for (_, child) in node.children.iter() {
            match child {
                Child::Tree(node) => self.read_layer(&node, layer + 1),
                Child::Leaf(data) => {
                    self.track_leaf_data_usage(&data);
                    leaf_children += 1;
                }
            }
        }

        self.leaf_child_counts
            .entry(layer)
            .and_modify(|v| v.push(leaf_children))
            .or_insert(vec![leaf_children]);
    }
}

impl core::fmt::Display for TreeInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "TreeInfo {{\n\testimated_size: {:?}\n\tdepth: {}\n\tnode_count: {}",
            self.estimated_size,
            self.depth,
            self.node_count(),
        ))?;

        let avg_prefix_length_map = self.avg_prefix_length();
        let max_prefix_length_map = self.max_prefix_length();
        let avg_child_count_map = self.avg_child_counts();
        let max_child_count_map = self.max_child_counts();
        let avg_leaf_child_count_map = self.avg_leaf_child_counts();
        let max_leaf_child_count_map = self.max_leaf_child_counts();
        let avg_leaf_nodes_map = self.avg_leaf_nodes();

        for (layer, node_count) in self.nodes.iter() {
            f.write_str("\n\tlayer #")?;
            f.write_str(&layer.to_string())?;
            f.write_char('\n')?;

            let avg_prefix_length = avg_prefix_length_map
                .get(layer)
                .cloned()
                .unwrap_or_default();

            let max_prefix_length = max_prefix_length_map
                .get(layer)
                .cloned()
                .unwrap_or_default();
            let avg_child_count = avg_child_count_map.get(layer).cloned().unwrap_or_default();
            let max_child_count = max_child_count_map.get(layer).cloned().unwrap_or_default();
            let avg_leaf_child_count = avg_leaf_child_count_map
                .get(layer)
                .cloned()
                .unwrap_or_default();
            let max_leaf_child_count = max_leaf_child_count_map
                .get(layer)
                .cloned()
                .unwrap_or_default();
            let avg_leaf_nodes = avg_leaf_nodes_map.get(layer).cloned().unwrap_or_default();

            f.write_fmt(format_args!("\t\tnode_count:\t{:.2}\n", node_count))?;
            f.write_fmt(format_args!(
                "\t\tμ prefix_len:\t{:.2}\n",
                avg_prefix_length
            ))?;
            f.write_fmt(format_args!(
                "\t\t^ prefix_len:\t{:.2}\n",
                max_prefix_length
            ))?;
            f.write_fmt(format_args!("\t\tμ child_count:\t{:.2}\n", avg_child_count))?;
            f.write_fmt(format_args!("\t\t^ child_count:\t{:.2}\n", max_child_count))?;
            f.write_fmt(format_args!(
                "\t\tμ lfc_count:\t{:.2}%\n",
                (avg_leaf_child_count as f64 / avg_child_count as f64) * 100.0
            ))?;
            f.write_fmt(format_args!(
                "\t\t^ lfc_count:\t{:.2}\n",
                max_leaf_child_count
            ))?;
            f.write_fmt(format_args!(
                "\t\tμ leaf_count:\t{:.2}%\n",
                avg_leaf_nodes * 100.0
            ))?;
        }

        f.write_char('}')?;

        Ok(())
    }
}
