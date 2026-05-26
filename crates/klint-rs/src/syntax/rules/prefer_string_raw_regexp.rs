use std::path::Path;
use tree_sitter::{Node, Parser};

use super::{PreferStringRawRegexpRecord, language_for_path, raw_node_text};

pub fn scan_prefer_string_raw_regexp(
    path: &Path,
    content: &str,
) -> Result<Vec<PreferStringRawRegexpRecord>, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&language_for_path(path))
        .map_err(|err| format!("klint-rs: failed to load TypeScript parser: {err}"))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| "klint-rs: failed to parse source".to_string())?;

    let root = tree.root_node();
    let mut records = Vec::new();
    walk_prefer_string_raw_regexp(root, content.as_bytes(), &mut records);
    Ok(records)
}
fn walk_prefer_string_raw_regexp(
    node: Node<'_>,
    source: &[u8],
    records: &mut Vec<PreferStringRawRegexpRecord>,
) {
    if node.kind() == "new_expression"
        && let Some(record) = prefer_string_raw_regexp_record(node, source)
    {
        records.push(record);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_prefer_string_raw_regexp(child, source, records);
    }
}
fn prefer_string_raw_regexp_record(
    node: Node<'_>,
    source: &[u8],
) -> Option<PreferStringRawRegexpRecord> {
    let constructor = node.child_by_field_name("constructor")?;
    if raw_node_text(constructor, source).as_deref() != Some("RegExp") {
        return None;
    }

    let arguments = node.child_by_field_name("arguments")?;
    let argument = first_named_argument(arguments)?;
    if argument.kind() != "template_string" {
        return None;
    }

    let raw = raw_node_text(argument, source)?;
    if !template_literal_chunks_have_double_backslash(&raw) {
        return None;
    }

    let start = argument.start_position();
    let end = argument.end_position();
    Some(PreferStringRawRegexpRecord {
        line: node.start_position().row + 1,
        fixed_arg: template_to_string_raw(&raw)?,
        start_row: start.row,
        end_row: end.row,
        start_byte: argument.start_byte(),
        end_byte: argument.end_byte(),
    })
}

fn template_literal_chunks_have_double_backslash(raw: &str) -> bool {
    scan_template_literal_chunks(raw, |chunk| chunk.contains("\\\\"))
}

fn template_to_string_raw(raw: &str) -> Option<String> {
    let inner = raw.strip_prefix('`')?.strip_suffix('`')?;
    let mut result = String::from("String.raw`");
    let mut index = 0;
    while index < inner.len() {
        let rest = &inner[index..];
        if rest.starts_with("${") {
            let end = template_expression_end(inner, index + 2)?;
            result.push_str(&inner[index..end]);
            index = end;
            continue;
        }
        if rest.starts_with("\\\\") {
            result.push('\\');
            index += 2;
            continue;
        }
        let ch = rest.chars().next()?;
        result.push(ch);
        index += ch.len_utf8();
    }
    result.push('`');
    Some(result)
}

fn scan_template_literal_chunks(raw: &str, mut predicate: impl FnMut(&str) -> bool) -> bool {
    let Some(inner) = raw
        .strip_prefix('`')
        .and_then(|value| value.strip_suffix('`'))
    else {
        return false;
    };

    let mut chunk_start = 0;
    let mut index = 0;
    while index < inner.len() {
        let rest = &inner[index..];
        if rest.starts_with("${") {
            if predicate(&inner[chunk_start..index]) {
                return true;
            }
            let Some(end) = template_expression_end(inner, index + 2) else {
                return false;
            };
            index = end;
            chunk_start = index;
            continue;
        }
        let Some(ch) = rest.chars().next() else {
            break;
        };
        index += ch.len_utf8();
    }
    predicate(&inner[chunk_start..])
}

fn template_expression_end(inner: &str, expression_start: usize) -> Option<usize> {
    let mut depth = 1;
    let mut index = expression_start;
    while index < inner.len() {
        let rest = &inner[index..];
        let ch = rest.chars().next()?;
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                return Some(index + 1);
            }
        }
        index += ch.len_utf8();
    }
    None
}
fn first_named_argument(arguments: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = arguments.walk();
    arguments
        .named_children(&mut cursor)
        .find(|child| child.kind() != "comment")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn extracts_prefer_string_raw_regexp_templates() {
        let records = scan_prefer_string_raw_regexp(
            &PathBuf::from("index.ts"),
            "const r = new RegExp(`\\\\.foo`);\ndeclare const n: string;\nconst s = new RegExp(`\\\\.(${n})`);\n",
        )
        .expect("source should parse");

        assert_eq!(
            records,
            vec![
                PreferStringRawRegexpRecord {
                    line: 1,
                    fixed_arg: "String.raw`\\.foo`".to_string(),
                    start_row: 0,
                    end_row: 0,
                    start_byte: 21,
                    end_byte: 29,
                },
                PreferStringRawRegexpRecord {
                    line: 3,
                    fixed_arg: "String.raw`\\.(${n})`".to_string(),
                    start_row: 2,
                    end_row: 2,
                    start_byte: 78,
                    end_byte: 89,
                },
            ]
        );
    }

    #[test]
    fn ignores_prefer_string_raw_regexp_without_template_backslashes() {
        let records = scan_prefer_string_raw_regexp(
            &PathBuf::from("index.ts"),
            "const a = new RegExp(`foo.bar`);\nconst b = new RegExp(/foo/);\nconst c = new RegExp(String.raw`\\.foo`);\nconst d = new RegExp(\"\\\\.foo\");\n",
        )
        .expect("source should parse");

        assert!(records.is_empty());
    }
}
