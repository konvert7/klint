use std::path::Path;
use tree_sitter::{Node, Parser};

use super::{is_jsx_path, language_for_path, node_text};

#[derive(Debug, PartialEq, Eq)]
pub struct ImportRecord {
    pub specifier: String,
    pub line: usize,
    pub is_type_only: bool,
    pub is_dynamic: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct JsxElementRecord {
    pub tag_name: String,
    pub line: usize,
}
pub fn scan_imports(path: &Path, content: &str) -> Result<Vec<ImportRecord>, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&language_for_path(path))
        .map_err(|err| format!("klint-rs: failed to load TypeScript parser: {err}"))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| "klint-rs: failed to parse source".to_string())?;

    let root = tree.root_node();
    let mut imports = Vec::new();
    walk_imports(root, content.as_bytes(), &mut imports);
    Ok(imports)
}

pub fn scan_jsx_elements(path: &Path, content: &str) -> Result<Vec<JsxElementRecord>, String> {
    if !is_jsx_path(path) {
        return Ok(Vec::new());
    }

    let mut parser = Parser::new();
    parser
        .set_language(&language_for_path(path))
        .map_err(|err| format!("klint-rs: failed to load TSX parser: {err}"))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| "klint-rs: failed to parse source".to_string())?;

    let root = tree.root_node();
    let mut elements = Vec::new();
    walk_jsx_elements(root, content.as_bytes(), &mut elements);
    Ok(elements)
}
fn walk_imports(node: Node<'_>, source: &[u8], imports: &mut Vec<ImportRecord>) {
    if node.kind() == "import_statement" {
        if let Some(record) = static_import_record(node, source) {
            imports.push(record);
        }
    } else if node.kind() == "call_expression"
        && let Some(record) = dynamic_import_record(node, source)
    {
        imports.push(record);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_imports(child, source, imports);
    }
}

fn static_import_record(node: Node<'_>, source: &[u8]) -> Option<ImportRecord> {
    let source_node = node.child_by_field_name("source")?;
    Some(ImportRecord {
        specifier: node_text(source_node, source)?,
        line: source_node.start_position().row + 1,
        is_type_only: static_import_is_type_only(node, source),
        is_dynamic: false,
    })
}

fn static_import_is_type_only(node: Node<'_>, source: &[u8]) -> bool {
    let Ok(text) = node.utf8_text(source) else {
        return false;
    };
    text.trim_start().starts_with("import type ")
}

fn dynamic_import_record(node: Node<'_>, source: &[u8]) -> Option<ImportRecord> {
    let function = node.child_by_field_name("function")?;
    if function.kind() != "import" {
        return None;
    }

    let arguments = node.child_by_field_name("arguments")?;
    let specifier = first_string_child(arguments)?;
    Some(ImportRecord {
        specifier: node_text(specifier, source)?,
        line: specifier.start_position().row + 1,
        is_type_only: false,
        is_dynamic: true,
    })
}

fn first_string_child(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| child.kind() == "string")
}

fn walk_jsx_elements(node: Node<'_>, source: &[u8], elements: &mut Vec<JsxElementRecord>) {
    if matches!(
        node.kind(),
        "jsx_opening_element" | "jsx_self_closing_element"
    ) && let Some(record) = jsx_element_record(node, source)
    {
        elements.push(record);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_jsx_elements(child, source, elements);
    }
}
fn jsx_element_record(node: Node<'_>, source: &[u8]) -> Option<JsxElementRecord> {
    let name = node
        .child_by_field_name("name")
        .or_else(|| first_identifier_child(node))?;
    if name.kind() != "identifier" {
        return None;
    }

    Some(JsxElementRecord {
        tag_name: node_text(name, source)?,
        line: name.start_position().row + 1,
    })
}

fn first_identifier_child(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| child.kind() == "identifier")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn imports(content: &str) -> Vec<ImportRecord> {
        scan_imports(&PathBuf::from("index.ts"), content).expect("source should parse")
    }

    #[test]
    fn extracts_static_imports_with_line_numbers() {
        assert_eq!(
            imports("import { foo } from \"./foo\";\nimport bar from '../bar';\n"),
            vec![
                ImportRecord {
                    specifier: "./foo".to_string(),
                    line: 1,
                    is_type_only: false,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "../bar".to_string(),
                    line: 2,
                    is_type_only: false,
                    is_dynamic: false,
                },
            ]
        );
    }

    #[test]
    fn extracts_dynamic_imports_with_line_numbers() {
        assert_eq!(
            imports("export async function load() {\n  return import(\"./lazy\");\n}\n"),
            vec![ImportRecord {
                specifier: "./lazy".to_string(),
                line: 2,
                is_type_only: false,
                is_dynamic: true,
            }]
        );
    }

    #[test]
    fn marks_type_only_imports() {
        assert_eq!(
            imports("import type { Foo } from \"./types\";\nimport { foo } from \"./foo\";\n"),
            vec![
                ImportRecord {
                    specifier: "./types".to_string(),
                    line: 1,
                    is_type_only: true,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "./foo".to_string(),
                    line: 2,
                    is_type_only: false,
                    is_dynamic: false,
                },
            ]
        );
    }

    #[test]
    fn uses_tsx_parser_for_tsx_files() {
        let records = scan_imports(
            &PathBuf::from("page.tsx"),
            "import { Button } from './button';\nexport const page = <Button />;\n",
        )
        .expect("tsx source should parse");

        assert_eq!(
            records,
            vec![ImportRecord {
                specifier: "./button".to_string(),
                line: 1,
                is_type_only: false,
                is_dynamic: false,
            }]
        );
    }

    #[test]
    fn extracts_jsx_opening_and_self_closing_elements() {
        let records = scan_jsx_elements(
            &PathBuf::from("page.tsx"),
            "export const page = <main>\n  <button>Click</button>\n  <input />\n</main>;\n",
        )
        .expect("tsx source should parse");

        assert_eq!(
            records,
            vec![
                JsxElementRecord {
                    tag_name: "main".to_string(),
                    line: 1,
                },
                JsxElementRecord {
                    tag_name: "button".to_string(),
                    line: 2,
                },
                JsxElementRecord {
                    tag_name: "input".to_string(),
                    line: 3,
                },
            ]
        );
    }

    #[test]
    fn skips_jsx_scan_for_plain_typescript_files() {
        let records = scan_jsx_elements(
            &PathBuf::from("page.ts"),
            "export const page = '<button>Click</button>';\n",
        )
        .expect("non-jsx source should be skipped");

        assert!(records.is_empty());
    }
}
