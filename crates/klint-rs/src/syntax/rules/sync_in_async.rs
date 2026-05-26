use std::path::Path;
use tree_sitter::{Node, Parser};

use super::{SyncInAsyncRecord, language_for_path, raw_node_text};

pub fn scan_sync_in_async(path: &Path, content: &str) -> Result<Vec<SyncInAsyncRecord>, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&language_for_path(path))
        .map_err(|err| format!("klint-rs: failed to load TypeScript parser: {err}"))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| "klint-rs: failed to parse source".to_string())?;

    let root = tree.root_node();
    let mut records = Vec::new();
    walk_sync_calls(root, false, content.as_bytes(), &mut records);
    Ok(records)
}
fn walk_sync_calls(
    node: Node<'_>,
    nearest_function_is_async: bool,
    source: &[u8],
    records: &mut Vec<SyncInAsyncRecord>,
) {
    let next_nearest_function_is_async = if is_function_like(node) {
        is_async_function_like(node, source)
    } else {
        nearest_function_is_async
    };

    if next_nearest_function_is_async && let Some(name) = sync_call_name(node, source) {
        records.push(SyncInAsyncRecord {
            line: node.start_position().row + 1,
            name,
        });
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_sync_calls(child, next_nearest_function_is_async, source, records);
    }
}
fn is_function_like(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "function_declaration" | "function_expression" | "arrow_function" | "method_definition"
    )
}

fn is_async_function_like(node: Node<'_>, source: &[u8]) -> bool {
    let Ok(text) = node.utf8_text(source) else {
        return false;
    };
    text.trim_start().starts_with("async ")
}

fn sync_call_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    if node.kind() != "call_expression" {
        return None;
    }

    let function = node.child_by_field_name("function")?;
    let name = if function.kind() == "identifier" {
        raw_node_text(function, source)?
    } else if function.kind() == "member_expression" {
        let property = function.child_by_field_name("property")?;
        raw_node_text(property, source)?
    } else {
        return None;
    };

    if name.ends_with("Sync") && name != "existsSync" {
        Some(name)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn extracts_sync_calls_inside_async_functions() {
        let records = scan_sync_in_async(
            &PathBuf::from("index.ts"),
            "async function load() {\n  readFileSync(path);\n  fs.writeFileSync(path, value);\n}\n",
        )
        .expect("source should parse");

        assert_eq!(
            records,
            vec![
                SyncInAsyncRecord {
                    line: 2,
                    name: "readFileSync".to_string(),
                },
                SyncInAsyncRecord {
                    line: 3,
                    name: "writeFileSync".to_string(),
                },
            ]
        );
    }

    #[test]
    fn ignores_exists_sync_and_nested_sync_functions() {
        let records = scan_sync_in_async(
            &PathBuf::from("index.ts"),
            "async function load() {\n  existsSync(path);\n  function nested() {\n    readFileSync(path);\n  }\n}\n",
        )
        .expect("source should parse");

        assert!(records.is_empty());
    }
}
