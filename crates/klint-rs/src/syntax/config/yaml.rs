use tree_sitter::Node;

use super::{Entries, joined, named_children};

const MAPPINGS: &[&str] = &["block_mapping", "flow_mapping"];
const PAIRS: &[&str] = &["block_mapping_pair", "flow_pair"];

pub(super) fn collect(root: Node<'_>, entries: &mut Entries<'_>) {
    for document in named_children(root) {
        walk(document, &[], entries);
    }
}

fn walk(node: Node<'_>, path: &[String], entries: &mut Entries<'_>) {
    match node.kind() {
        kind if MAPPINGS.contains(&kind) => {
            for pair in named_children(node)
                .into_iter()
                .filter(|child| PAIRS.contains(&child.kind()))
            {
                walk_pair(pair, path, entries);
            }
        }
        "block_sequence" | "flow_sequence" => walk_items(node, path, entries),
        _ => {
            for child in named_children(node) {
                walk(child, path, entries);
            }
        }
    }
}

fn walk_items(sequence: Node<'_>, path: &[String], entries: &mut Entries<'_>) {
    let items = named_children(sequence)
        .into_iter()
        .filter(|child| matches!(child.kind(), "block_sequence_item" | "flow_node"));

    for (index, item) in items.enumerate() {
        let item_path = joined(path, index.to_string());
        let marker = item
            .child(0)
            .filter(|_| item.kind() == "block_sequence_item");
        entries.push(&item_path, marker.unwrap_or(item), scalar(item, entries));
        walk(item, &item_path, entries);
    }
}

fn walk_pair(pair: Node<'_>, path: &[String], entries: &mut Entries<'_>) {
    let Some(key) = pair.child_by_field_name("key") else {
        return;
    };
    let Some(name) = scalar(key, entries) else {
        return;
    };
    let value = pair.child_by_field_name("value");
    let pair_path = joined(path, name);
    let text = value.and_then(|value| scalar(value, entries));
    entries.push(&pair_path, key, text);

    if let Some(value) = value {
        walk(value, &pair_path, entries);
    }
}

fn scalar(node: Node<'_>, entries: &Entries<'_>) -> Option<String> {
    match node.kind() {
        "plain_scalar" | "block_scalar" => Some(entries.text(node)),
        "single_quote_scalar" | "double_quote_scalar" => Some(entries.unquoted(node)),
        "flow_node" | "block_node" | "block_sequence_item" => {
            let children = named_children(node);
            let [only] = children.as_slice() else {
                return None;
            };
            scalar(*only, entries)
        }
        _ => None,
    }
}
