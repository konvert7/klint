use std::path::Path;
use tree_sitter::{Node, Parser};

use super::{PreferStringRawRecord, language_for_path, raw_node_text};

pub fn scan_prefer_string_raw(
    path: &Path,
    content: &str,
) -> Result<Vec<PreferStringRawRecord>, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&language_for_path(path))
        .map_err(|err| format!("klint-rs: failed to load TypeScript parser: {err}"))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| "klint-rs: failed to parse source".to_string())?;

    let root = tree.root_node();
    let mut records = Vec::new();
    walk_prefer_string_raw(root, content.as_bytes(), &mut records);
    Ok(records)
}

fn walk_prefer_string_raw(node: Node<'_>, source: &[u8], records: &mut Vec<PreferStringRawRecord>) {
    if node.kind() == "string"
        && let Some(record) = prefer_string_raw_record(node, source)
    {
        records.push(record);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_prefer_string_raw(child, source, records);
    }
}

fn prefer_string_raw_record(node: Node<'_>, source: &[u8]) -> Option<PreferStringRawRecord> {
    let raw = raw_node_text(node, source)?;
    if !raw.contains("\\\\") {
        return None;
    }

    let value = string_literal_value(&raw)?;
    if value.contains('`') || value.contains("${") || value.ends_with('\\') {
        return None;
    }

    let start = node.start_position();
    let end = node.end_position();
    Some(PreferStringRawRecord {
        line: start.row + 1,
        fixed: format!("String.raw`{value}`"),
        start_row: start.row,
        end_row: end.row,
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
    })
}

fn string_literal_value(raw: &str) -> Option<String> {
    let quote = raw.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    if !raw.ends_with(quote) {
        return None;
    }

    let inner = raw.get(1..raw.len() - 1)?;
    let mut result = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            result.push(ch);
            continue;
        }

        let escaped = chars.next()?;
        match escaped {
            '\\' => result.push('\\'),
            '"' => result.push('"'),
            '\'' => result.push('\''),
            '`' => result.push('`'),
            'n' => result.push('\n'),
            'r' => result.push('\r'),
            't' => result.push('\t'),
            'b' => result.push('\u{0008}'),
            'f' => result.push('\u{000c}'),
            'v' => result.push('\u{000b}'),
            '0' => result.push('\0'),
            other => result.push(other),
        }
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn extracts_string_literals_with_escaped_backslashes() {
        let records = scan_prefer_string_raw(
            &PathBuf::from("index.ts"),
            r#"const p = "C:\\Users\\Documents\\file.txt";"#,
        )
        .expect("source should parse");

        assert_eq!(
            records,
            vec![PreferStringRawRecord {
                line: 1,
                fixed: r#"String.raw`C:\Users\Documents\file.txt`"#.to_string(),
                start_row: 0,
                end_row: 0,
                start_byte: 10,
                end_byte: 42,
            }]
        );
    }

    #[test]
    fn ignores_string_literals_that_cannot_use_raw_templates() {
        let records = scan_prefer_string_raw(
            &PathBuf::from("index.ts"),
            r#"const a = "hello";
const b = "foo\\bar`baz";
const c = "C:\\Users${foo}";
const d = "trailing\\";
"#,
        )
        .expect("source should parse");

        assert!(records.is_empty());
    }
}
