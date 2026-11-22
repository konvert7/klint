use std::collections::HashMap;
use tree_sitter::Node;

use super::{Entries, joined, named_children};

const KEYS: &[&str] = &["bare_key", "quoted_key", "dotted_key"];

pub(super) fn collect(root: Node<'_>, entries: &mut Entries<'_>) {
    let mut array_tables: HashMap<Vec<String>, usize> = HashMap::new();

    for child in named_children(root) {
        match child.kind() {
            "pair" => walk_pair(child, &[], entries),
            "table" => walk_table(child, None, entries),
            "table_array_element" => walk_table(child, Some(&mut array_tables), entries),
            _ => {}
        }
    }
}

fn walk_table(
    table: Node<'_>,
    array_tables: Option<&mut HashMap<Vec<String>, usize>>,
    entries: &mut Entries<'_>,
) {
    let children = named_children(table);
    let Some(header) = children.iter().find(|child| KEYS.contains(&child.kind())) else {
        return;
    };
    let mut path = key_path(*header, entries);

    if let Some(counts) = array_tables {
        let count = counts.entry(path.clone()).or_insert(0);
        path.push(count.to_string());
        *count += 1;
    }

    entries.push(&path, *header, None);

    for pair in children.iter().filter(|child| child.kind() == "pair") {
        walk_pair(*pair, &path, entries);
    }
}

fn walk_pair(pair: Node<'_>, path: &[String], entries: &mut Entries<'_>) {
    let children = named_children(pair);
    let [key, value, ..] = children.as_slice() else {
        return;
    };
    let pair_path = [path, &key_path(*key, entries)].concat();
    entries.push(&pair_path, *key, scalar(*value, entries));
    walk_value(*value, &pair_path, entries);
}

fn walk_value(value: Node<'_>, path: &[String], entries: &mut Entries<'_>) {
    match value.kind() {
        "inline_table" => {
            for pair in named_children(value)
                .into_iter()
                .filter(|child| child.kind() == "pair")
            {
                walk_pair(pair, path, entries);
            }
        }
        "array" => {
            let items = named_children(value)
                .into_iter()
                .filter(|child| child.kind() != "comment");

            for (index, item) in items.enumerate() {
                let item_path = joined(path, index.to_string());
                entries.push(&item_path, item, scalar(item, entries));
                walk_value(item, &item_path, entries);
            }
        }
        _ => {}
    }
}

fn key_path(key: Node<'_>, entries: &Entries<'_>) -> Vec<String> {
    match key.kind() {
        "dotted_key" => named_children(key)
            .into_iter()
            .flat_map(|part| key_path(part, entries))
            .collect(),
        "quoted_key" => vec![entries.unquoted(key)],
        _ => vec![entries.text(key)],
    }
}

fn scalar(value: Node<'_>, entries: &Entries<'_>) -> Option<String> {
    match value.kind() {
        "string" => Some(entries.unquoted(value)),
        "inline_table" | "array" => None,
        _ => Some(entries.text(value)),
    }
}
