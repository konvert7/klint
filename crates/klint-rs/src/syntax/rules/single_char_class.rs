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
        .map_err(|err| format!("klint: failed to load TypeScript parser: {err}"))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| "klint: failed to parse source".to_string())?;

    Ok(scan_single_char_classes_from_tree(
        tree.root_node(),
        content.as_bytes(),
    ))
}

pub(crate) fn scan_single_char_classes_from_tree(
    root: Node<'_>,
    source: &[u8],
) -> Vec<SingleCharClassRecord> {
    let mut records = Vec::new();
    walk_single_char_classes(root, source, &mut records);
    records
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

pub(crate) fn single_char_class_record(
    node: Node<'_>,
    source: &[u8],
) -> Option<SingleCharClassRecord> {
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
        match byte_at(pattern, i) {
            Some(b'\\') => {
                i += 2;
                continue;
            }
            Some(b'[') => {}
            _ => {
                i += 1;
                continue;
            }
        }

        let start = i;

        if byte_at(pattern, i + 1) == Some(b'^') {
            i = after_negated_class(pattern, i + 1);
            continue;
        }

        let (mut tokens, end) = class_tokens(pattern, i + 1);
        if end >= pattern.len() {
            break;
        }

        i = end + 1;

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

fn after_negated_class(pattern: &str, mut i: usize) -> usize {
    while i < pattern.len() && byte_at(pattern, i) != Some(b']') {
        if byte_at(pattern, i) == Some(b'\\') {
            i += 1;
        }

        i += 1;
    }

    i + 1
}

fn class_tokens(pattern: &str, mut i: usize) -> (Vec<String>, usize) {
    let mut tokens = Vec::new();

    while i < pattern.len() && byte_at(pattern, i) != Some(b']') {
        let Some(next) = token_end(pattern, i) else {
            return (tokens, pattern.len());
        };

        tokens.push(pattern[i..next].to_string());
        i = next;
    }

    (tokens, i)
}

fn token_end(pattern: &str, i: usize) -> Option<usize> {
    if byte_at(pattern, i) != Some(b'\\') {
        return pattern[i..].chars().next().map(|ch| i + ch.len_utf8());
    }

    let i = i + 1;
    let len = pattern.len();

    Some(match byte_at(pattern, i)? {
        b'u' if i + 5 <= len => i + 5,
        b'x' if i + 3 <= len => i + 3,
        b'c' if i + 1 < len => i + 2,
        _ => i + 1,
    })
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
