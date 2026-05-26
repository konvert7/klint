use std::path::Path;
use tree_sitter::{Node, Parser};

use super::{ConsecutiveArrayPushRecord, language_for_path, raw_node_text};

pub fn scan_consecutive_array_push(
    path: &Path,
    content: &str,
) -> Result<Vec<ConsecutiveArrayPushRecord>, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&language_for_path(path))
        .map_err(|err| format!("klint-rs: failed to load TypeScript parser: {err}"))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| "klint-rs: failed to parse source".to_string())?;

    let root = tree.root_node();
    let mut records = Vec::new();
    walk_statement_containers(root, content.as_bytes(), &mut records);
    Ok(records)
}
fn walk_statement_containers(
    node: Node<'_>,
    source: &[u8],
    records: &mut Vec<ConsecutiveArrayPushRecord>,
) {
    if matches!(node.kind(), "program" | "statement_block") {
        scan_statement_run(node, source, records);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_statement_containers(child, source, records);
    }
}

fn scan_statement_run(
    node: Node<'_>,
    source: &[u8],
    records: &mut Vec<ConsecutiveArrayPushRecord>,
) {
    let mut run_start: Option<Node<'_>> = None;
    let mut run_receiver = String::new();
    let mut run_count = 0;

    let flush = |records: &mut Vec<ConsecutiveArrayPushRecord>,
                 run_start: &mut Option<Node<'_>>,
                 run_receiver: &mut String,
                 run_count: &mut usize| {
        if let Some(start) = run_start
            && *run_count >= 2
        {
            records.push(ConsecutiveArrayPushRecord {
                line: start.start_position().row + 1,
                count: *run_count,
                receiver: run_receiver.clone(),
            });
        }
        *run_start = None;
        run_receiver.clear();
        *run_count = 0;
    };

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        let receiver = push_receiver(child, source);
        if let Some(receiver) = receiver {
            if receiver == run_receiver {
                run_count += 1;
            } else {
                flush(records, &mut run_start, &mut run_receiver, &mut run_count);
                run_start = Some(child);
                run_receiver = receiver;
                run_count = 1;
            }
        } else {
            flush(records, &mut run_start, &mut run_receiver, &mut run_count);
        }
    }
    flush(records, &mut run_start, &mut run_receiver, &mut run_count);
}

fn push_receiver(node: Node<'_>, source: &[u8]) -> Option<String> {
    if node.kind() != "expression_statement" {
        return None;
    }

    let expression = node.named_child(0)?;
    if expression.kind() != "call_expression" {
        return None;
    }

    let function = expression.child_by_field_name("function")?;
    if function.kind() != "member_expression" {
        return None;
    }

    let property = function.child_by_field_name("property")?;
    if raw_node_text(property, source)? != "push" {
        return None;
    }

    function
        .child_by_field_name("object")
        .and_then(|object| raw_node_text(object, source))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn extracts_consecutive_array_push_runs() {
        let records = scan_consecutive_array_push(
            &PathBuf::from("index.ts"),
            "const arr: number[] = [];\narr.push(1);\narr.push(2);\narr.push(3);\n",
        )
        .expect("source should parse");

        assert_eq!(
            records,
            vec![ConsecutiveArrayPushRecord {
                line: 2,
                count: 3,
                receiver: "arr".to_string(),
            }]
        );
    }

    #[test]
    fn ignores_array_push_runs_split_by_other_statements() {
        let records = scan_consecutive_array_push(
            &PathBuf::from("index.ts"),
            "const arr: number[] = [];\narr.push(1);\nconsole.log(arr);\narr.push(2);\n",
        )
        .expect("source should parse");

        assert!(records.is_empty());
    }
}
