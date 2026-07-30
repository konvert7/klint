use tree_sitter::Node;

use super::ImportRecord;
use crate::syntax::raw_node_text;

pub(super) fn walk_rust_imports(node: Node<'_>, source: &[u8], imports: &mut Vec<ImportRecord>) {
    if node.kind() == "use_declaration" {
        imports.extend(rust_use_records(node, source));
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_rust_imports(child, source, imports);
    }
}

fn rust_use_records(node: Node<'_>, source: &[u8]) -> Vec<ImportRecord> {
    let Some(argument) = node.child_by_field_name("argument") else {
        return Vec::new();
    };
    let mut paths = Vec::new();
    collect_rust_use_paths(argument, source, "", &mut paths);
    let line = node.start_position().row + 1;
    paths
        .into_iter()
        .map(|specifier| ImportRecord {
            specifier,
            line,
            is_type_only: false,
            is_dynamic: false,
        })
        .collect()
}

/// Flattens one `use` tree into a path per imported target. A braced list
/// multiplies its prefix across every branch, so `use a::{b::{c, d}, e as f}`
/// yields `a::b::c`, `a::b::d`, and `a::e` — the same per-target treatment
/// Python multi-target imports get. A bare `self` in a list names the prefix
/// itself, and `as` aliases and `*` wildcards contribute their path only.
fn collect_rust_use_paths(node: Node<'_>, source: &[u8], prefix: &str, paths: &mut Vec<String>) {
    match node.kind() {
        "scoped_use_list" => {
            let nested = match node.child_by_field_name("path") {
                Some(path) => {
                    join_rust_path(prefix, &raw_node_text(path, source).unwrap_or_default())
                }
                None => prefix.to_string(),
            };
            let Some(list) = node.child_by_field_name("list") else {
                return;
            };
            let mut cursor = list.walk();
            for child in list.named_children(&mut cursor) {
                collect_rust_use_paths(child, source, &nested, paths);
            }
        }
        "use_as_clause" => {
            if let Some(path) = node.child_by_field_name("path") {
                collect_rust_use_paths(path, source, prefix, paths);
            }
        }
        "use_wildcard" => {
            let mut cursor = node.walk();
            if let Some(path) = node.named_children(&mut cursor).next() {
                collect_rust_use_paths(path, source, prefix, paths);
            }
        }
        "self" if !prefix.is_empty() => paths.push(prefix.to_string()),
        _ => {
            if let Some(text) = raw_node_text(node, source) {
                paths.push(join_rust_path(prefix, &text));
            }
        }
    }
}

fn join_rust_path(prefix: &str, segment: &str) -> String {
    if prefix.is_empty() {
        segment.to_string()
    } else {
        format!("{prefix}::{segment}")
    }
}

#[cfg(test)]
mod tests {
    use crate::syntax::scan_imports;
    use std::path::PathBuf;

    #[test]
    fn flattens_rust_use_trees_into_one_record_per_target() {
        let records = scan_imports(
            &PathBuf::from("crates/klint-rs/src/arch.rs"),
            "use crate::syntax::{TreeCache, scan_imports_from_tree};\nuse std::collections::{BTreeMap, BTreeSet};\nuse super::helper as alias;\nuse self::inner::*;\nuse crate::files::{self, normalize_path};\n// use crate::ignored::Thing;\n",
        )
        .expect("rust source should parse");

        assert_eq!(
            records
                .iter()
                .map(|record| (record.specifier.as_str(), record.line))
                .collect::<Vec<_>>(),
            vec![
                ("crate::syntax::TreeCache", 1),
                ("crate::syntax::scan_imports_from_tree", 1),
                ("std::collections::BTreeMap", 2),
                ("std::collections::BTreeSet", 2),
                ("super::helper", 3),
                ("self::inner", 4),
                ("crate::files", 5),
                ("crate::files::normalize_path", 5),
            ]
        );
    }
}
