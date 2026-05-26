use std::path::Path;
use tree_sitter::{Node, Parser};

use super::{SingleCharClassRecord, language_for_path, raw_node_text};

pub fn scan_single_char_classes(
    path: &Path,
    content: &str,
) -> Result<Vec<SingleCharClassRecord>, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&language_for_path(path))
        .map_err(|err| format!("klint-rs: failed to load TypeScript parser: {err}"))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| "klint-rs: failed to parse source".to_string())?;

    let root = tree.root_node();
    let mut records = Vec::new();
    walk_single_char_classes(root, content.as_bytes(), &mut records);
    Ok(records)
}
fn walk_single_char_classes(
    node: Node<'_>,
    source: &[u8],
    records: &mut Vec<SingleCharClassRecord>,
) {
    if node.kind() == "regex"
        && let Some(record) = single_char_class_record(node, source)
    {
        records.push(record);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_single_char_classes(child, source, records);
    }
}
#[derive(Debug)]
struct CharClass {
    start: usize,
    end: usize,
    inner: String,
}

fn single_char_class_record(node: Node<'_>, source: &[u8]) -> Option<SingleCharClassRecord> {
    let regex = raw_node_text(node, source)?;
    let pattern_end = regex.rfind('/')?;
    let pattern = regex.get(1..pattern_end)?;
    let flags = regex.get(pattern_end + 1..)?;
    let classes = parse_single_char_classes(pattern);
    let to_fix = classes
        .iter()
        .filter(|class| !is_char_class_metachar_exception(&class.inner))
        .collect::<Vec<_>>();
    if to_fix.is_empty() {
        return None;
    }

    let mut fixed_pattern = String::new();
    let mut prev = 0;
    for class in &classes {
        if is_char_class_metachar_exception(&class.inner) {
            continue;
        }
        fixed_pattern.push_str(pattern.get(prev..class.start)?);
        fixed_pattern.push_str(&class.inner);
        prev = class.end + 1;
    }
    fixed_pattern.push_str(pattern.get(prev..)?);

    let start = node.start_position();
    let end = node.end_position();
    Some(SingleCharClassRecord {
        line: start.row + 1,
        class: to_fix[0].inner.clone(),
        fixed_regex: format!("/{fixed_pattern}/{flags}"),
        start_row: start.row,
        end_row: end.row,
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
    })
}
fn parse_single_char_classes(pattern: &str) -> Vec<CharClass> {
    let mut results = Vec::new();
    let mut i = 0;
    while i < pattern.len() {
        if byte_at(pattern, i) == Some(b'\\') {
            i += 2;
            continue;
        }
        if byte_at(pattern, i) != Some(b'[') {
            i += 1;
            continue;
        }

        let start = i;
        i += 1;
        if byte_at(pattern, i) == Some(b'^') {
            while i < pattern.len() && byte_at(pattern, i) != Some(b']') {
                if byte_at(pattern, i) == Some(b'\\') {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
            continue;
        }

        let mut tokens = Vec::new();
        while i < pattern.len() && byte_at(pattern, i) != Some(b']') {
            if byte_at(pattern, i) == Some(b'\\') {
                let token_start = i;
                i += 1;
                if i >= pattern.len() {
                    break;
                }
                match byte_at(pattern, i) {
                    Some(b'u') if i + 5 <= pattern.len() => i += 5,
                    Some(b'x') if i + 3 <= pattern.len() => i += 3,
                    Some(b'c') if i + 1 < pattern.len() => i += 2,
                    Some(_) => i += 1,
                    None => break,
                }
                tokens.push(pattern[token_start..i].to_string());
            } else {
                let Some(ch) = pattern[i..].chars().next() else {
                    break;
                };
                let token_start = i;
                i += ch.len_utf8();
                tokens.push(pattern[token_start..i].to_string());
            }
        }

        if i >= pattern.len() {
            break;
        }
        let end = i;
        i += 1;
        if tokens.len() == 1 {
            results.push(CharClass {
                start,
                end,
                inner: tokens.remove(0),
            });
        }
    }
    results
}

fn byte_at(value: &str, index: usize) -> Option<u8> {
    value.as_bytes().get(index).copied()
}

fn is_char_class_metachar_exception(inner: &str) -> bool {
    matches!(
        inner,
        "." | "*" | "+" | "?" | "{" | "}" | "(" | ")" | "|" | "$"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn extracts_single_char_classes() {
        let records =
            scan_single_char_classes(&PathBuf::from("index.ts"), "const r = /a[b]c[\\d]/;\n")
                .expect("source should parse");

        assert_eq!(
            records,
            vec![SingleCharClassRecord {
                line: 1,
                class: "b".to_string(),
                fixed_regex: "/abc\\d/".to_string(),
                start_row: 0,
                end_row: 0,
                start_byte: 10,
                end_byte: 21,
            }]
        );
    }

    #[test]
    fn ignores_non_single_and_exception_classes() {
        let records = scan_single_char_classes(
            &PathBuf::from("index.ts"),
            "const r = /[ab][a-z][^a][.][*]/;\n",
        )
        .expect("source should parse");

        assert!(records.is_empty());
    }
}
