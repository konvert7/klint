use std::path::Path;
use tree_sitter::{Node, Parser};

use super::{NestedTemplateLiteralRecord, language_for_path};

pub fn scan_nested_template_literals(
    path: &Path,
    content: &str,
) -> Result<Vec<NestedTemplateLiteralRecord>, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&language_for_path(path))
        .map_err(|err| format!("klint-rs: failed to load TypeScript parser: {err}"))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| "klint-rs: failed to parse source".to_string())?;

    let root = tree.root_node();
    let mut records = Vec::new();
    walk_template_literals(root, &mut records);
    Ok(records)
}
fn walk_template_literals(node: Node<'_>, records: &mut Vec<NestedTemplateLiteralRecord>) {
    if node.kind() == "template_string" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "template_substitution" {
                find_nested_template_literals(child, records);
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_template_literals(child, records);
    }
}

fn find_nested_template_literals(node: Node<'_>, records: &mut Vec<NestedTemplateLiteralRecord>) {
    if is_tagged_template_call(node) {
        return;
    }

    if node.kind() == "template_string" {
        records.push(NestedTemplateLiteralRecord {
            line: node.start_position().row + 1,
        });
        return;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        find_nested_template_literals(child, records);
    }
}

fn is_tagged_template_call(node: Node<'_>) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }

    let mut cursor = node.walk();
    node.children(&mut cursor)
        .any(|child| child.kind() == "template_string")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn extracts_nested_template_literals() {
        let records = scan_nested_template_literals(
            &PathBuf::from("index.ts"),
            "declare const b: boolean;\nconst value = `${b ? `yes` : `no`}`;\n",
        )
        .expect("source should parse");

        assert_eq!(
            records,
            vec![
                NestedTemplateLiteralRecord { line: 2 },
                NestedTemplateLiteralRecord { line: 2 },
            ]
        );
    }

    #[test]
    fn ignores_standalone_and_tagged_template_literals() {
        let records = scan_nested_template_literals(
            &PathBuf::from("index.ts"),
            "declare function tag(s: TemplateStringsArray): string;\nconst standalone = `hello`;\nconst value = `${tag`inner`}`;\n",
        )
        .expect("source should parse");

        assert!(records.is_empty());
    }
}
