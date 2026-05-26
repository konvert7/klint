use std::path::Path;
use tree_sitter::{Node, Parser};

use super::{PreferNullishCoalescingAssignRecord, language_for_path, raw_node_text};

pub fn scan_prefer_nullish_coalescing_assign(
    path: &Path,
    content: &str,
) -> Result<Vec<PreferNullishCoalescingAssignRecord>, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&language_for_path(path))
        .map_err(|err| format!("klint-rs: failed to load TypeScript parser: {err}"))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| "klint-rs: failed to parse source".to_string())?;

    let root = tree.root_node();
    let mut records = Vec::new();
    walk_prefer_nullish_coalescing_assign(root, content.as_bytes(), &mut records);
    Ok(records)
}

fn walk_prefer_nullish_coalescing_assign(
    node: Node<'_>,
    source: &[u8],
    records: &mut Vec<PreferNullishCoalescingAssignRecord>,
) {
    if node.kind() == "if_statement"
        && let Some(record) = prefer_nullish_coalescing_assign_record(node, source)
    {
        records.push(record);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_prefer_nullish_coalescing_assign(child, source, records);
    }
}

fn prefer_nullish_coalescing_assign_record(
    node: Node<'_>,
    source: &[u8],
) -> Option<PreferNullishCoalescingAssignRecord> {
    if node.child_by_field_name("alternative").is_some() {
        return None;
    }

    let condition = node.child_by_field_name("condition")?;
    let target = nullish_guard_target(condition, source)?;
    let assignment = assignment_expression(node.child_by_field_name("consequence")?, source)?;
    if assignment.target != target {
        return None;
    }

    let start = node.start_position();
    let end = node.end_position();
    Some(PreferNullishCoalescingAssignRecord {
        line: start.row + 1,
        target,
        value: assignment.value,
        start_row: start.row,
        end_row: end.row,
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
    })
}

fn assignment_expression(
    node: Node<'_>,
    source: &[u8],
) -> Option<PreferNullishCoalescingAssignTarget> {
    let expression = if node.kind() == "expression_statement" {
        node.named_child(0)?
    } else if node.kind() == "statement_block" && node.named_child_count() == 1 {
        let child = node.named_child(0)?;
        if child.kind() != "expression_statement" {
            return None;
        }
        child.named_child(0)?
    } else {
        return None;
    };

    if expression.kind() != "assignment_expression" || !node_has_child_kind(expression, "=") {
        return None;
    }

    let left = expression.child_by_field_name("left")?;
    let right = expression.child_by_field_name("right")?;
    Some(PreferNullishCoalescingAssignTarget {
        target: raw_node_text(left, source)?,
        value: raw_node_text(right, source)?,
    })
}

fn nullish_guard_target(node: Node<'_>, source: &[u8]) -> Option<String> {
    if node.kind() == "parenthesized_expression" {
        return node
            .named_child(0)
            .and_then(|child| nullish_guard_target(child, source));
    }

    if let Some(direct) = equality_nullish_target(node, source) {
        return Some(direct.target);
    }

    if node.kind() != "binary_expression" || !node_has_child_kind(node, "||") {
        return None;
    }

    let left = node
        .child_by_field_name("left")
        .and_then(|child| equality_nullish_target(child, source))?;
    let right = node
        .child_by_field_name("right")
        .and_then(|child| equality_nullish_target(child, source))?;
    if left.target != right.target {
        return None;
    }

    if matches!(
        (left.value.as_str(), right.value.as_str()),
        ("null", "undefined") | ("undefined", "null")
    ) {
        Some(left.target)
    } else {
        None
    }
}

fn equality_nullish_target(
    node: Node<'_>,
    source: &[u8],
) -> Option<PreferNullishCoalescingAssignTarget> {
    if node.kind() != "binary_expression"
        || !(node_has_child_kind(node, "==") || node_has_child_kind(node, "==="))
    {
        return None;
    }

    let left = node.child_by_field_name("left")?;
    let right = node.child_by_field_name("right")?;
    let left_text = raw_node_text(left, source)?;
    let right_text = raw_node_text(right, source)?;

    if right_text == "null" || right_text == "undefined" {
        return Some(PreferNullishCoalescingAssignTarget {
            target: left_text,
            value: right_text,
        });
    }
    if left_text == "null" || left_text == "undefined" {
        return Some(PreferNullishCoalescingAssignTarget {
            target: right_text,
            value: left_text,
        });
    }
    None
}

fn node_has_child_kind(node: Node<'_>, kind: &str) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|child| child.kind() == kind)
}

#[derive(Debug)]
struct PreferNullishCoalescingAssignTarget {
    target: String,
    value: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn extracts_direct_nullish_assignment_guards() {
        let records = scan_prefer_nullish_coalescing_assign(
            &PathBuf::from("index.ts"),
            "let x: object | undefined;\nif (x == null) x = {};\nif (result.hooks == null) result.hooks = {};\n",
        )
        .expect("source should parse");

        assert_eq!(
            records,
            vec![
                PreferNullishCoalescingAssignRecord {
                    line: 2,
                    target: "x".to_string(),
                    value: "{}".to_string(),
                    start_row: 1,
                    end_row: 1,
                    start_byte: 27,
                    end_byte: 49,
                },
                PreferNullishCoalescingAssignRecord {
                    line: 3,
                    target: "result.hooks".to_string(),
                    value: "{}".to_string(),
                    start_row: 2,
                    end_row: 2,
                    start_byte: 50,
                    end_byte: 94,
                },
            ]
        );
    }

    #[test]
    fn extracts_strict_null_or_undefined_guards_and_blocks() {
        let records = scan_prefer_nullish_coalescing_assign(
            &PathBuf::from("index.ts"),
            "if (x === null || x === undefined) { x = {}; }\nif (undefined === y || null === y) y = fallback;\n",
        )
        .expect("source should parse");

        assert_eq!(
            records,
            vec![
                PreferNullishCoalescingAssignRecord {
                    line: 1,
                    target: "x".to_string(),
                    value: "{}".to_string(),
                    start_row: 0,
                    end_row: 0,
                    start_byte: 0,
                    end_byte: 46,
                },
                PreferNullishCoalescingAssignRecord {
                    line: 2,
                    target: "y".to_string(),
                    value: "fallback".to_string(),
                    start_row: 1,
                    end_row: 1,
                    start_byte: 47,
                    end_byte: 95,
                },
            ]
        );
    }

    #[test]
    fn ignores_unsafe_or_non_matching_guards() {
        let records = scan_prefer_nullish_coalescing_assign(
            &PathBuf::from("index.ts"),
            "if (!x) x = {};\nif (x == null) y = {};\nif (x === null || x === false) x = {};\nif (x == null) x = {}; else doThing();\nx ??= {};\n",
        )
        .expect("source should parse");

        assert!(records.is_empty());
    }
}
