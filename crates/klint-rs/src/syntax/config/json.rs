use tree_sitter::Node;

use super::{Entries, joined, named_children};

pub(super) fn collect(root: Node<'_>, entries: &mut Entries<'_>) {
    for value in named_children(root) {
        walk(value, &[], entries);
    }
}

fn walk(node: Node<'_>, path: &[String], entries: &mut Entries<'_>) {
    match node.kind() {
        "object" => {
            for pair in named_children(node)
                .into_iter()
                .filter(|child| child.kind() == "pair")
            {
                walk_pair(pair, path, entries);
            }
        }
        "array" => {
            let items = named_children(node)
                .into_iter()
                .filter(|child| child.kind() != "comment");

            for (index, item) in items.enumerate() {
                let item_path = joined(path, index.to_string());
                entries.push(&item_path, item, scalar(item, entries));
                walk(item, &item_path, entries);
            }
        }
        _ => {}
    }
}

fn walk_pair(pair: Node<'_>, path: &[String], entries: &mut Entries<'_>) {
    let Some(key) = pair.child_by_field_name("key") else {
        return;
    };
    let value = pair.child_by_field_name("value");
    let pair_path = joined(path, entries.unquoted(key));
    let text = value.and_then(|value| scalar(value, entries));
    entries.push(&pair_path, key, text);

    if let Some(value) = value {
        walk(value, &pair_path, entries);
    }
}

fn scalar(node: Node<'_>, entries: &Entries<'_>) -> Option<String> {
    match node.kind() {
        "string" => Some(entries.unquoted(node)),
        "number" | "true" | "false" | "null" => Some(entries.text(node)),
        _ => None,
    }
}
