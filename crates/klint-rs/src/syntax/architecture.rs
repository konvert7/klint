use std::path::Path;
use tree_sitter::{Node, Parser};

use super::{SourceLanguage, is_jsx_path, language_for_path, node_text, source_language_for_path};

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
    if source_language_for_path(path) == SourceLanguage::Swift {
        return Ok(swift_imports(content));
    }

    let mut parser = Parser::new();
    parser
        .set_language(&language_for_path(path))
        .map_err(|err| format!("klint-rs: failed to load TypeScript parser: {err}"))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| "klint-rs: failed to parse source".to_string())?;

    let root = tree.root_node();
    let mut imports = Vec::new();
    match source_language_for_path(path) {
        SourceLanguage::Python => walk_python_imports(root, content.as_bytes(), &mut imports),
        SourceLanguage::Swift => imports.extend(swift_imports(content)),
        SourceLanguage::JavaScriptLike => walk_imports(root, content.as_bytes(), &mut imports),
    }
    Ok(imports)
}

/// Same scan as [`scan_imports`] but reuses an already-parsed tree. The
/// caller is responsible for only passing trees for non-Swift paths (Swift
/// import scanning never parses with tree-sitter — see [`scan_imports`]).
pub(crate) fn scan_imports_from_tree(
    path: &Path,
    root: Node<'_>,
    source: &[u8],
) -> Vec<ImportRecord> {
    let mut imports = Vec::new();
    match source_language_for_path(path) {
        SourceLanguage::Python => walk_python_imports(root, source, &mut imports),
        SourceLanguage::Swift => {
            imports.extend(swift_imports(&String::from_utf8_lossy(source)));
        }
        SourceLanguage::JavaScriptLike => walk_imports(root, source, &mut imports),
    }
    imports
}

fn swift_imports(content: &str) -> Vec<ImportRecord> {
    content
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            swift_import_specifier(line).map(|specifier| ImportRecord {
                specifier,
                line: index + 1,
                is_type_only: false,
                is_dynamic: false,
            })
        })
        .collect()
}

fn swift_import_specifier(line: &str) -> Option<String> {
    let mut text = line.trim();
    if text.starts_with("//") {
        return None;
    }

    while let Some(rest) = text.strip_prefix('@') {
        let attr_end = rest.find(char::is_whitespace)?;
        text = rest[attr_end..].trim_start();
    }

    let rest = text.strip_prefix("import ")?;
    let mut parts = rest.split_whitespace();
    let mut target = parts.next()?;
    if matches!(
        target,
        "class" | "struct" | "enum" | "protocol" | "func" | "typealias" | "var" | "let"
    ) {
        target = parts.next()?;
    }

    let module = target
        .split('.')
        .next()?
        .trim_matches(|char: char| !char.is_alphanumeric() && char != '_');
    if module.is_empty() {
        None
    } else {
        Some(module.to_string())
    }
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

    Ok(scan_jsx_elements_from_tree(
        tree.root_node(),
        content.as_bytes(),
    ))
}

/// Same scan as [`scan_jsx_elements`] but reuses an already-parsed tree. The
/// caller is expected to only pass jsx-path files (non-jsx grammars never
/// produce jsx node kinds, so this is a no-op for them either way).
pub(crate) fn scan_jsx_elements_from_tree(root: Node<'_>, source: &[u8]) -> Vec<JsxElementRecord> {
    let mut elements = Vec::new();
    walk_jsx_elements(root, source, &mut elements);
    elements
}

#[derive(Debug, PartialEq, Eq)]
pub struct CommentRecord {
    /// 1-based first physical line the comment occupies.
    pub start_line: usize,
    /// 1-based last physical line the comment occupies.
    pub end_line: usize,
    pub is_doc: bool,
}

/// Collects every comment node (all grammars name them `comment` — `//`,
/// `/* */`, and Python `#`), classified as doc vs ordinary. Docstrings are
/// string expressions, not comment nodes, so they never appear here.
pub(crate) fn scan_comments_from_tree(root: Node<'_>, source: &[u8]) -> Vec<CommentRecord> {
    let mut comments = Vec::new();
    walk_comments(root, source, &mut comments);
    comments
}

fn walk_comments(node: Node<'_>, source: &[u8], comments: &mut Vec<CommentRecord>) {
    if node.kind() == "comment" {
        let text = node.utf8_text(source).unwrap_or("");
        comments.push(CommentRecord {
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
            is_doc: is_doc_comment(text),
        });
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_comments(child, source, comments);
    }
}

/// A `/** */` JSDoc block, but not the empty `/**/` comment. Mirrors the TS engine.
fn is_doc_comment(text: &str) -> bool {
    text.starts_with("/**") && text != "/**/"
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

fn walk_python_imports(node: Node<'_>, source: &[u8], imports: &mut Vec<ImportRecord>) {
    match node.kind() {
        "import_statement" => imports.extend(python_import_records(node, source)),
        "import_from_statement" => imports.extend(python_from_import_records(node, source)),
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_python_imports(child, source, imports);
    }
}

fn python_import_records(node: Node<'_>, source: &[u8]) -> Vec<ImportRecord> {
    python_import_names(node)
        .into_iter()
        .filter_map(|name| {
            Some(python_record(python_import_target_text(name, source)?, name))
        })
        .collect()
}

fn python_from_import_records(node: Node<'_>, source: &[u8]) -> Vec<ImportRecord> {
    let Some(module) = node.child_by_field_name("module_name") else {
        return Vec::new();
    };

    if module.kind() != "relative_import" {
        return node_text(module, source)
            .map(|specifier| vec![python_record(specifier, module)])
            .unwrap_or_default();
    }

    let Some(prefix) = python_relative_prefix(module, source) else {
        return Vec::new();
    };

    if let Some(module_path) = python_relative_module_path(module, source) {
        return vec![python_record(format!("{prefix}{module_path}"), module)];
    }

    python_import_names(node)
        .into_iter()
        .filter_map(|name| {
            let target = python_import_target_text(name, source)?;
            Some(python_record(format!("{prefix}{target}"), name))
        })
        .collect()
}

fn python_import_names<'tree>(node: Node<'tree>) -> Vec<Node<'tree>> {
    let mut cursor = node.walk();
    node.children_by_field_name("name", &mut cursor).collect()
}

fn python_import_target_text(name: Node<'_>, source: &[u8]) -> Option<String> {
    let target = if name.kind() == "aliased_import" {
        name.child_by_field_name("name")?
    } else {
        name
    };
    node_text(target, source)
}

fn python_relative_prefix(module: Node<'_>, source: &[u8]) -> Option<String> {
    let mut cursor = module.walk();
    let prefix = module
        .children(&mut cursor)
        .find(|child| child.kind() == "import_prefix")?;
    let dots = node_text(prefix, source)?
        .chars()
        .filter(|char| *char == '.')
        .count();
    Some(if dots == 1 {
        "./".to_string()
    } else {
        "../".repeat(dots - 1)
    })
}

fn python_relative_module_path(module: Node<'_>, source: &[u8]) -> Option<String> {
    let mut cursor = module.walk();
    let dotted = module
        .children(&mut cursor)
        .find(|child| child.kind() == "dotted_name")?;
    Some(node_text(dotted, source)?.replace('.', "/"))
}

fn python_record(specifier: String, node: Node<'_>) -> ImportRecord {
    ImportRecord {
        specifier,
        line: node.start_position().row + 1,
        is_type_only: false,
        is_dynamic: false,
    }
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
    use std::path::{Path, PathBuf};
    use tree_sitter::Parser;

    fn imports(content: &str) -> Vec<ImportRecord> {
        scan_imports(&PathBuf::from("index.ts"), content).expect("source should parse")
    }

    fn comments(path: &str, content: &str) -> Vec<CommentRecord> {
        let mut parser = Parser::new();
        parser
            .set_language(&language_for_path(Path::new(path)))
            .expect("parser loads");
        let tree = parser.parse(content, None).expect("source parses");
        scan_comments_from_tree(tree.root_node(), content.as_bytes())
    }

    #[test]
    fn classifies_doc_and_ordinary_comments_with_line_spans() {
        assert_eq!(
            comments(
                "index.ts",
                "// line\nconst x = 1; /* inline */\n/**\n * doc\n */\n",
            ),
            vec![
                CommentRecord {
                    start_line: 1,
                    end_line: 1,
                    is_doc: false,
                },
                CommentRecord {
                    start_line: 2,
                    end_line: 2,
                    is_doc: false,
                },
                CommentRecord {
                    start_line: 3,
                    end_line: 5,
                    is_doc: true,
                },
            ]
        );
    }

    #[test]
    fn treats_python_hash_comments_as_ordinary() {
        assert_eq!(
            comments("mod.py", "# a\nx = 1\n"),
            vec![CommentRecord {
                start_line: 1,
                end_line: 1,
                is_doc: false,
            }]
        );
    }

    #[test]
    fn ignores_the_empty_block_comment() {
        assert_eq!(
            comments("index.ts", "/**/\n"),
            vec![CommentRecord {
                start_line: 1,
                end_line: 1,
                is_doc: false,
            }]
        );
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
    fn extracts_python_relative_imports_with_line_numbers() {
        let records = scan_imports(
            &PathBuf::from("src/jobs/worker.py"),
            "import requests\nfrom ..lib.auth import load_key\nfrom . import local\n",
        )
        .expect("python source should parse");

        assert_eq!(
            records,
            vec![
                ImportRecord {
                    specifier: "requests".to_string(),
                    line: 1,
                    is_type_only: false,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "../lib/auth".to_string(),
                    line: 2,
                    is_type_only: false,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "./local".to_string(),
                    line: 3,
                    is_type_only: false,
                    is_dynamic: false,
                },
            ]
        );
    }

    #[test]
    fn extracts_every_target_of_a_multi_target_python_import() {
        let records = scan_imports(
            &PathBuf::from("src/jobs/worker.py"),
            "import os, sys\nimport os.path as p, json\n",
        )
        .expect("python source should parse");

        assert_eq!(
            records,
            vec![
                ImportRecord {
                    specifier: "os".to_string(),
                    line: 1,
                    is_type_only: false,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "sys".to_string(),
                    line: 1,
                    is_type_only: false,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "os.path".to_string(),
                    line: 2,
                    is_type_only: false,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "json".to_string(),
                    line: 2,
                    is_type_only: false,
                    is_dynamic: false,
                },
            ]
        );
    }

    #[test]
    fn extracts_every_target_of_a_bare_relative_python_import() {
        let records = scan_imports(
            &PathBuf::from("src/jobs/worker.py"),
            "from . import first, second as alias\nfrom ..lib import (\n    auth,\n    other,\n)\n",
        )
        .expect("python source should parse");

        assert_eq!(
            records,
            vec![
                ImportRecord {
                    specifier: "./first".to_string(),
                    line: 1,
                    is_type_only: false,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "./second".to_string(),
                    line: 1,
                    is_type_only: false,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "../lib".to_string(),
                    line: 2,
                    is_type_only: false,
                    is_dynamic: false,
                },
            ]
        );
    }

    #[test]
    fn ignores_python_wildcard_imports_without_a_module_path() {
        let records = scan_imports(
            &PathBuf::from("src/jobs/worker.py"),
            "from . import *\nfrom app.lib import *\n",
        )
        .expect("python source should parse");

        assert_eq!(
            records,
            vec![ImportRecord {
                specifier: "app.lib".to_string(),
                line: 2,
                is_type_only: false,
                is_dynamic: false,
            }]
        );
    }

    #[test]
    fn extracts_swift_imports_with_line_numbers() {
        let records = scan_imports(
            &PathBuf::from("Sources/App/UI/ViewModel.swift"),
            "import Foundation\n@_exported import Core\nimport struct Models.User\n// import Ignored\n",
        )
        .expect("swift imports should scan");

        assert_eq!(
            records,
            vec![
                ImportRecord {
                    specifier: "Foundation".to_string(),
                    line: 1,
                    is_type_only: false,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "Core".to_string(),
                    line: 2,
                    is_type_only: false,
                    is_dynamic: false,
                },
                ImportRecord {
                    specifier: "Models".to_string(),
                    line: 3,
                    is_type_only: false,
                    is_dynamic: false,
                },
            ]
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
