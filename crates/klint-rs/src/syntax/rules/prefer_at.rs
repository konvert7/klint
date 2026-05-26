use std::path::Path;
use tree_sitter::{Node, Parser};

use super::{PreferAtRecord, language_for_path, raw_node_text};

pub fn scan_prefer_at(path: &Path, content: &str) -> Result<Vec<PreferAtRecord>, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&language_for_path(path))
        .map_err(|err| format!("klint-rs: failed to load TypeScript parser: {err}"))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| "klint-rs: failed to parse source".to_string())?;

    let root = tree.root_node();
    let mut records = Vec::new();
    walk_prefer_at(root, content.as_bytes(), &mut records);
    Ok(records)
}
fn walk_prefer_at(node: Node<'_>, source: &[u8], records: &mut Vec<PreferAtRecord>) {
    if node.kind() == "subscript_expression"
        && let Some(record) = prefer_at_record(node, source)
    {
        records.push(record);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_prefer_at(child, source, records);
    }
}
fn prefer_at_record(node: Node<'_>, source: &[u8]) -> Option<PreferAtRecord> {
    let object = node.child_by_field_name("object")?;
    let index = node.child_by_field_name("index")?;
    if index.kind() != "binary_expression" {
        return None;
    }

    let left = index.child_by_field_name("left")?;
    let right = index.child_by_field_name("right")?;
    if !binary_expression_has_operator(index, "-") {
        return None;
    }
    if left.kind() != "member_expression" {
        return None;
    }

    let property = left.child_by_field_name("property")?;
    if raw_node_text(property, source).as_deref() != Some("length") {
        return None;
    }

    let offset = numeric_literal_integer_value(right, source)?;
    if offset <= 0 {
        return None;
    }

    let base = raw_node_text(object, source)?;
    let length_base = left
        .child_by_field_name("object")
        .and_then(|base| raw_node_text(base, source))?;
    if base != length_base {
        return None;
    }

    let start = node.start_position();
    let end = node.end_position();
    Some(PreferAtRecord {
        line: start.row + 1,
        base,
        offset,
        start_row: start.row,
        end_row: end.row,
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
    })
}
fn binary_expression_has_operator(node: Node<'_>, operator: &str) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .any(|child| child.kind() == operator)
}

fn numeric_literal_integer_value(node: Node<'_>, source: &[u8]) -> Option<i64> {
    if node.kind() != "number" {
        return None;
    }
    let value = raw_node_text(node, source)?;
    let parsed = value.parse::<f64>().ok()?;
    if !parsed.is_finite() || parsed.fract() != 0.0 {
        return None;
    }
    Some(parsed as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn extracts_prefer_at_negative_index_access() {
        let records = scan_prefer_at(
            &PathBuf::from("index.ts"),
            "const x = arr[arr.length - 1];\nconst y = obj.items[obj.items.length - 5];\n",
        )
        .expect("source should parse");

        assert_eq!(
            records,
            vec![
                PreferAtRecord {
                    line: 1,
                    base: "arr".to_string(),
                    offset: 1,
                    start_row: 0,
                    end_row: 0,
                    start_byte: 10,
                    end_byte: 29,
                },
                PreferAtRecord {
                    line: 2,
                    base: "obj.items".to_string(),
                    offset: 5,
                    start_row: 1,
                    end_row: 1,
                    start_byte: 41,
                    end_byte: 72,
                },
            ]
        );
    }

    #[test]
    fn ignores_prefer_at_non_matching_index_access() {
        let records = scan_prefer_at(
            &PathBuf::from("index.ts"),
            "const a = arr[other.length - 1];\nconst b = arr[arr.length - 0];\nconst c = arr[arr.length + 1];\nconst d = arr[i];\n",
        )
        .expect("source should parse");

        assert!(records.is_empty());
    }
}
