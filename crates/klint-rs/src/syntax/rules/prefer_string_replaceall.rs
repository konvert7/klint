use std::path::Path;
use tree_sitter::{Node, Parser};

use super::{PreferStringReplaceAllRecord, language_for_path, raw_node_text};

pub fn scan_prefer_string_replaceall(
    path: &Path,
    content: &str,
) -> Result<Vec<PreferStringReplaceAllRecord>, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&language_for_path(path))
        .map_err(|err| format!("klint-rs: failed to load TypeScript parser: {err}"))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| "klint-rs: failed to parse source".to_string())?;

    let root = tree.root_node();
    let mut records = Vec::new();
    walk_prefer_string_replaceall(root, content.as_bytes(), &mut records);
    Ok(records)
}
fn walk_prefer_string_replaceall(
    node: Node<'_>,
    source: &[u8],
    records: &mut Vec<PreferStringReplaceAllRecord>,
) {
    if node.kind() == "call_expression"
        && let Some(record) = prefer_string_replaceall_record(node, source)
    {
        records.push(record);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_prefer_string_replaceall(child, source, records);
    }
}
fn prefer_string_replaceall_record(
    node: Node<'_>,
    source: &[u8],
) -> Option<PreferStringReplaceAllRecord> {
    let function = node.child_by_field_name("function")?;
    if function.kind() != "member_expression" {
        return None;
    }

    let property = function.child_by_field_name("property")?;
    if raw_node_text(property, source).as_deref() != Some("replace") {
        return None;
    }

    let arguments = node.child_by_field_name("arguments")?;
    let mut cursor = arguments.walk();
    let args = arguments
        .named_children(&mut cursor)
        .filter(|child| child.kind() != "comment")
        .collect::<Vec<_>>();
    if args.len() != 2 || args[0].kind() != "regex" {
        return None;
    }

    let regex = raw_node_text(args[0], source)?;
    let flags = regex_flags(&regex);
    if flags != "g" {
        return None;
    }
    let pattern = regex_pattern(&regex)?;
    if !is_plain_regex_literal(pattern) {
        return None;
    }

    let receiver = function
        .child_by_field_name("object")
        .and_then(|object| raw_node_text(object, source))?;
    let replacement = raw_node_text(args[1], source)?;
    let pattern_lit = string_literal_for_pattern(pattern);
    let start = node.start_position();
    let end = node.end_position();

    Some(PreferStringReplaceAllRecord {
        line: start.row + 1,
        receiver,
        pattern: pattern.to_string(),
        pattern_lit,
        replacement,
        start_row: start.row,
        end_row: end.row,
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
    })
}
fn regex_pattern(literal: &str) -> Option<&str> {
    let index = literal.rfind('/')?;
    literal.get(1..index)
}

fn is_plain_regex_literal(pattern: &str) -> bool {
    let stripped = pattern.replace("\\\\", "");
    !stripped.chars().any(|ch| {
        matches!(
            ch,
            '.' | '*' | '+' | '?' | '[' | ']' | '{' | '}' | '(' | ')' | '|' | '^' | '$' | '\\'
        )
    })
}

fn string_literal_for_pattern(pattern: &str) -> String {
    if pattern.contains('"') {
        format!("'{}'", pattern.replace('\'', "\\'"))
    } else {
        format!("\"{pattern}\"")
    }
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
    fn extracts_prefer_string_replaceall_plain_global_regex() {
        let records = scan_prefer_string_replaceall(
            &PathBuf::from("index.ts"),
            "const r = text.replace(/foo/g, repl);\nconst path2 = path.replace(/\\\\/g, \"/\");\n",
        )
        .expect("source should parse");

        assert_eq!(
            records,
            vec![
                PreferStringReplaceAllRecord {
                    line: 1,
                    receiver: "text".to_string(),
                    pattern: "foo".to_string(),
                    pattern_lit: "\"foo\"".to_string(),
                    replacement: "repl".to_string(),
                    start_row: 0,
                    end_row: 0,
                    start_byte: 10,
                    end_byte: 36,
                },
                PreferStringReplaceAllRecord {
                    line: 2,
                    receiver: "path".to_string(),
                    pattern: "\\\\".to_string(),
                    pattern_lit: "\"\\\\\"".to_string(),
                    replacement: "\"/\"".to_string(),
                    start_row: 1,
                    end_row: 1,
                    start_byte: 52,
                    end_byte: 76,
                },
            ]
        );
    }

    #[test]
    fn ignores_prefer_string_replaceall_non_plain_regex() {
        let records = scan_prefer_string_replaceall(
            &PathBuf::from("index.ts"),
            "a.replace(/foo/gi, x);\nb.replace(/./g, x);\nc.replace(/\\./g, x);\nd.replace(/foo/, x);\n",
        )
        .expect("source should parse");

        assert!(records.is_empty());
    }
}
