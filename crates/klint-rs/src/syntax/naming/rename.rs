use std::ops::Range;

use tree_sitter::Node;

use super::Rename;

const REFERENCE_KINDS: &[&str] = &["identifier", "type_identifier"];
const NAME_KINDS: &[&str] = &[
    "identifier",
    "type_identifier",
    "property_identifier",
    "shorthand_property_identifier",
    "shorthand_property_identifier_pattern",
    "private_property_identifier",
];
const SHORTHAND_PROPERTY: &str = "shorthand_property_identifier";
const EXPORT_KINDS: &[&str] = &["export_statement", "export_specifier", "export_clause"];

pub(super) fn same_file(
    declaration: Node<'_>,
    old_name: &str,
    new_name: &str,
    source: &[u8],
) -> Option<Rename> {
    if is_exported(declaration) {
        return None;
    }

    let mut edits = Vec::new();
    let mut nodes = vec![root_of(declaration)];

    while let Some(node) = nodes.pop() {
        let mut cursor = node.walk();
        nodes.extend(node.children(&mut cursor));

        if !NAME_KINDS.contains(&node.kind()) {
            continue;
        }

        let text = node.utf8_text(source).ok()?;
        if text == new_name || (text == old_name && blocks_rename(node)) {
            return None;
        }

        if text == old_name {
            edits.extend(edit_for(node, old_name, new_name));
        }
    }

    edits.sort_by_key(|(range, _)| range.start);
    Some(Rename { edits })
}

fn is_exported(declaration: Node<'_>) -> bool {
    declaration
        .parent()
        .is_some_and(|parent| parent.kind() == "export_statement")
}

fn edit_for(node: Node<'_>, old_name: &str, new_name: &str) -> Option<(Range<usize>, String)> {
    match node.kind() {
        SHORTHAND_PROPERTY => Some((node.byte_range(), format!("{old_name}: {new_name}"))),
        kind if REFERENCE_KINDS.contains(&kind) => Some((node.byte_range(), new_name.to_string())),
        _ => None,
    }
}

fn blocks_rename(node: Node<'_>) -> bool {
    node.kind() == "shorthand_property_identifier_pattern"
        || node
            .parent()
            .is_some_and(|parent| EXPORT_KINDS.contains(&parent.kind()))
}

fn root_of(node: Node<'_>) -> Node<'_> {
    let mut root = node;

    while let Some(parent) = root.parent() {
        root = parent;
    }

    root
}
