use std::path::Path;
use tree_sitter::{Node, Parser};

use super::{UnguardedJsonParseRecord, language_for_path, raw_node_text};

pub fn scan_unguarded_json_parse(
    path: &Path,
    content: &str,
) -> Result<Vec<UnguardedJsonParseRecord>, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&language_for_path(path))
        .map_err(|err| format!("klint-rs: failed to load TypeScript parser: {err}"))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| "klint-rs: failed to parse source".to_string())?;

    let root = tree.root_node();
    let mut records = Vec::new();
    walk_json_parse_calls(root, false, content.as_bytes(), &mut records);
    Ok(records)
}
fn walk_json_parse_calls(
    node: Node<'_>,
    inside_try: bool,
    source: &[u8],
    records: &mut Vec<UnguardedJsonParseRecord>,
) {
    let next_inside_try = inside_try || node.kind() == "try_statement";
    if !next_inside_try && is_json_parse_call(node, source) {
        records.push(UnguardedJsonParseRecord {
            line: node.start_position().row + 1,
        });
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_json_parse_calls(child, next_inside_try, source, records);
    }
}

fn is_json_parse_call(node: Node<'_>, source: &[u8]) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }

    let Some(function) = node.child_by_field_name("function") else {
        return false;
    };
    if function.kind() != "member_expression" {
        return false;
    }

    let Some(object) = function.child_by_field_name("object") else {
        return false;
    };
    let Some(property) = function.child_by_field_name("property") else {
        return false;
    };

    raw_node_text(object, source).as_deref() == Some("JSON")
        && raw_node_text(property, source).as_deref() == Some("parse")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn extracts_unguarded_json_parse_calls() {
        let records = scan_unguarded_json_parse(
            &PathBuf::from("index.ts"),
            "const value = JSON.parse(raw);\ntry {\n  JSON.parse(raw);\n} catch {}\n",
        )
        .expect("source should parse");

        assert_eq!(records, vec![UnguardedJsonParseRecord { line: 1 }]);
    }

    #[test]
    fn ignores_json_parse_nested_inside_try_statement() {
        let records = scan_unguarded_json_parse(
            &PathBuf::from("index.ts"),
            "try {\n  function parse() {\n    return JSON.parse(raw);\n  }\n} catch {}\n",
        )
        .expect("source should parse");

        assert!(records.is_empty());
    }
}
