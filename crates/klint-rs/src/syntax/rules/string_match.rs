use std::path::Path;
use tree_sitter::{Node, Parser};

use super::{StringMatchRecord, language_for_path, raw_node_text};

pub fn scan_string_match(path: &Path, content: &str) -> Result<Vec<StringMatchRecord>, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&language_for_path(path))
        .map_err(|err| format!("klint-rs: failed to load TypeScript parser: {err}"))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| "klint-rs: failed to parse source".to_string())?;

    let root = tree.root_node();
    let mut records = Vec::new();
    walk_string_match(root, content.as_bytes(), &mut records);
    Ok(records)
}
fn walk_string_match(node: Node<'_>, source: &[u8], records: &mut Vec<StringMatchRecord>) {
    if node.kind() == "call_expression"
        && let Some(record) = string_match_record(node, source)
    {
        records.push(record);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_string_match(child, source, records);
    }
}
fn string_match_record(node: Node<'_>, source: &[u8]) -> Option<StringMatchRecord> {
    let function = node.child_by_field_name("function")?;
    if function.kind() != "member_expression" {
        return None;
    }

    let property = function.child_by_field_name("property")?;
    if raw_node_text(property, source)? != "match" {
        return None;
    }

    let arguments = node.child_by_field_name("arguments")?;
    let argument = single_named_argument(arguments)?;
    if argument.kind() != "regex" {
        return None;
    }

    let regex = raw_node_text(argument, source)?;
    if regex_flags(&regex).contains('g') {
        return None;
    }

    let receiver = function
        .child_by_field_name("object")
        .and_then(|object| raw_node_text(object, source))?;
    let start = node.start_position();
    let end = node.end_position();

    Some(StringMatchRecord {
        line: start.row + 1,
        receiver,
        regex,
        start_row: start.row,
        end_row: end.row,
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
    })
}
fn single_named_argument(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    let mut arguments = node
        .named_children(&mut cursor)
        .filter(|child| child.kind() != "comment");
    let argument = arguments.next()?;
    if arguments.next().is_some() {
        return None;
    }
    Some(argument)
}

fn regex_flags(literal: &str) -> &str {
    let Some(index) = literal.rfind('/') else {
        return "";
    };
    &literal[index + 1..]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn extracts_non_global_string_match_calls() {
        let records = scan_string_match(
            &PathBuf::from("index.ts"),
            "const m = line.match(/foo/i);\nconst ok = line.match(/foo/g);\n",
        )
        .expect("source should parse");

        assert_eq!(
            records,
            vec![StringMatchRecord {
                line: 1,
                receiver: "line".to_string(),
                regex: "/foo/i".to_string(),
                start_row: 0,
                end_row: 0,
                start_byte: 10,
                end_byte: 28,
            }]
        );
    }

    #[test]
    fn ignores_string_match_with_variable_argument() {
        let records = scan_string_match(
            &PathBuf::from("index.ts"),
            "declare const re: RegExp;\nconst m = line.match(re);\n",
        )
        .expect("source should parse");

        assert!(records.is_empty());
    }
}
