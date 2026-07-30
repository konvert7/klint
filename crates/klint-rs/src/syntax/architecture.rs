use std::collections::BTreeSet;
use std::path::Path;
use tree_sitter::{Node, Parser};

use super::{
    SourceLanguage, is_jsx_path, language_for_path, node_text, raw_node_text,
    source_language_for_path,
};

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

    Ok(scan_imports_from_tree(
        path,
        tree.root_node(),
        content.as_bytes(),
    ))
}

/// Same scan as [`scan_imports`] but reuses an already-parsed tree.
pub(crate) fn scan_imports_from_tree(
    path: &Path,
    root: Node<'_>,
    source: &[u8],
) -> Vec<ImportRecord> {
    let mut imports = Vec::new();
    match source_language_for_path(path) {
        SourceLanguage::Python => imports.extend(scan_python_imports(root, source)),
        SourceLanguage::Swift => walk_swift_imports(root, source, &mut imports),
        SourceLanguage::Rust => walk_rust_imports(root, source, &mut imports),
        SourceLanguage::JavaScriptLike => walk_imports(root, source, &mut imports),
    }
    imports
}

fn walk_swift_imports(node: Node<'_>, source: &[u8], imports: &mut Vec<ImportRecord>) {
    if node.kind() == "import_declaration" {
        imports.extend(swift_import_record(node, source));
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_swift_imports(child, source, imports);
    }
}

/// The module a Swift `import` names. `import_declaration` exposes no field
/// names, so the module is the first `simple_identifier` under its `identifier`
/// child — one shape for `import Beta.Sub` and `import struct Models.User`
/// alike, with `@testable`/`@_exported` parked in a sibling `modifiers` node.
fn swift_import_record(node: Node<'_>, source: &[u8]) -> Option<ImportRecord> {
    let identifier = first_named_child_of_kind(node, "identifier")?;
    let module = first_named_child_of_kind(identifier, "simple_identifier")?;
    Some(ImportRecord {
        specifier: raw_node_text(module, source)?,
        line: identifier.start_position().row + 1,
        is_type_only: false,
        is_dynamic: false,
    })
}

fn first_named_child_of_kind<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|child| child.kind() == kind)
}

fn walk_rust_imports(node: Node<'_>, source: &[u8], imports: &mut Vec<ImportRecord>) {
    if node.kind() == "use_declaration" {
        imports.extend(rust_use_records(node, source));
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_rust_imports(child, source, imports);
    }
}

fn rust_use_records(node: Node<'_>, source: &[u8]) -> Vec<ImportRecord> {
    let Some(argument) = node.child_by_field_name("argument") else {
        return Vec::new();
    };
    let mut paths = Vec::new();
    collect_rust_use_paths(argument, source, "", &mut paths);
    let line = node.start_position().row + 1;
    paths
        .into_iter()
        .map(|specifier| ImportRecord {
            specifier,
            line,
            is_type_only: false,
            is_dynamic: false,
        })
        .collect()
}

/// Flattens one `use` tree into a path per imported target. A braced list
/// multiplies its prefix across every branch, so `use a::{b::{c, d}, e as f}`
/// yields `a::b::c`, `a::b::d`, and `a::e` — the same per-target treatment
/// Python multi-target imports get. A bare `self` in a list names the prefix
/// itself, and `as` aliases and `*` wildcards contribute their path only.
fn collect_rust_use_paths(node: Node<'_>, source: &[u8], prefix: &str, paths: &mut Vec<String>) {
    match node.kind() {
        "scoped_use_list" => {
            let nested = match node.child_by_field_name("path") {
                Some(path) => {
                    join_rust_path(prefix, &raw_node_text(path, source).unwrap_or_default())
                }
                None => prefix.to_string(),
            };
            let Some(list) = node.child_by_field_name("list") else {
                return;
            };
            let mut cursor = list.walk();
            for child in list.named_children(&mut cursor) {
                collect_rust_use_paths(child, source, &nested, paths);
            }
        }
        "use_as_clause" => {
            if let Some(path) = node.child_by_field_name("path") {
                collect_rust_use_paths(path, source, prefix, paths);
            }
        }
        "use_wildcard" => {
            let mut cursor = node.walk();
            if let Some(path) = node.named_children(&mut cursor).next() {
                collect_rust_use_paths(path, source, prefix, paths);
            }
        }
        "self" if !prefix.is_empty() => paths.push(prefix.to_string()),
        _ => {
            if let Some(text) = raw_node_text(node, source) {
                paths.push(join_rust_path(prefix, &text));
            }
        }
    }
}

fn join_rust_path(prefix: &str, segment: &str) -> String {
    if prefix.is_empty() {
        segment.to_string()
    } else {
        format!("{prefix}::{segment}")
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

/// Collects every comment node, classified as doc vs ordinary. Each grammar
/// names comments differently: `comment` in TypeScript and Python,
/// `comment`/`multiline_comment` in Swift, `line_comment`/`block_comment` in
/// Rust. Docstrings are string expressions, not comment nodes, so they never
/// appear here.
pub(crate) fn scan_comments_from_tree(
    path: &Path,
    root: Node<'_>,
    source: &[u8],
) -> Vec<CommentRecord> {
    let mut comments = Vec::new();
    walk_comments(source_language_for_path(path), root, source, &mut comments);
    comments
}

/// Rows the comment text itself covers, counted from its first row. Rust
/// doc-comment nodes extend past their last character to swallow the newline,
/// so the node's own end row would credit them with a line they do not occupy.
fn trailing_line_span(text: &str) -> usize {
    text.trim_end().lines().count().saturating_sub(1)
}

fn walk_comments(
    language: SourceLanguage,
    node: Node<'_>,
    source: &[u8],
    comments: &mut Vec<CommentRecord>,
) {
    if matches!(
        node.kind(),
        "comment" | "multiline_comment" | "line_comment" | "block_comment"
    ) {
        let text = node.utf8_text(source).unwrap_or("");
        let start_line = node.start_position().row + 1;
        comments.push(CommentRecord {
            start_line,
            end_line: start_line + trailing_line_span(text),
            is_doc: is_doc_comment(language, node, text),
        });
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_comments(language, child, source, comments);
    }
}

/// A `/** */` JSDoc block, but not the empty `/**/` comment. Mirrors the TS
/// engine. Swift also documents with `///`, where TypeScript reserves triple
/// slashes for `/// <reference>` directives rather than documentation. Rust
/// marks `///` and `//!` in the tree itself, so its doc comments are read off
/// the node rather than guessed from the text.
fn is_doc_comment(language: SourceLanguage, node: Node<'_>, text: &str) -> bool {
    match language {
        SourceLanguage::Rust => has_rust_doc_marker(node),
        SourceLanguage::Swift if text.starts_with("///") => true,
        _ => text.starts_with("/**") && text != "/**/",
    }
}

fn has_rust_doc_marker(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).any(|child| {
        matches!(
            child.kind(),
            "outer_doc_comment_marker" | "inner_doc_comment_marker"
        )
    })
}
fn walk_imports(node: Node<'_>, source: &[u8], imports: &mut Vec<ImportRecord>) {
    match node.kind() {
        "import_statement" => imports.extend(module_source_record(node, source, "import type ")),
        "export_statement" => imports.extend(module_source_record(node, source, "export type ")),
        "call_expression" => imports.extend(dynamic_import_record(node, source)),
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_imports(child, source, imports);
    }
}

/// The specifier of a statement that names another module — `import … from
/// "x"` and re-exporting `export … from "x"`. A local `export const` carries no
/// `source` field, so it yields nothing.
fn module_source_record(
    node: Node<'_>,
    source: &[u8],
    type_only_prefix: &str,
) -> Option<ImportRecord> {
    let source_node = node.child_by_field_name("source")?;
    Some(ImportRecord {
        specifier: node_text(source_node, source)?,
        line: source_node.start_position().row + 1,
        is_type_only: node_starts_with(node, source, type_only_prefix),
        is_dynamic: false,
    })
}

fn node_starts_with(node: Node<'_>, source: &[u8], prefix: &str) -> bool {
    let Ok(text) = node.utf8_text(source) else {
        return false;
    };
    text.trim_start().starts_with(prefix)
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

fn scan_python_imports(root: Node<'_>, source: &[u8]) -> Vec<ImportRecord> {
    let mut imports = Vec::new();
    let importlib = python_importlib_bindings(root, source);
    walk_python_imports(root, source, &importlib, false, &mut imports);
    imports
}

fn walk_python_imports(
    node: Node<'_>,
    source: &[u8],
    importlib: &ImportlibBindings,
    type_only: bool,
    imports: &mut Vec<ImportRecord>,
) {
    match node.kind() {
        "import_statement" => {
            imports.extend(with_type_only(
                python_import_records(node, source),
                type_only,
            ));
        }
        "import_from_statement" => {
            imports.extend(with_type_only(
                python_from_import_records(node, source),
                type_only,
            ));
        }
        "call" => {
            imports.extend(with_type_only(
                python_dynamic_import_record(node, source, importlib),
                type_only,
            ));
        }
        _ => {}
    }

    let guarded_branch = guards_type_checking(node, source)
        .then(|| node.child_by_field_name("consequence"))
        .flatten();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let child_type_only =
            type_only || guarded_branch.is_some_and(|branch| branch.id() == child.id());
        walk_python_imports(child, source, importlib, child_type_only, imports);
    }
}

fn with_type_only(
    records: impl IntoIterator<Item = ImportRecord>,
    type_only: bool,
) -> impl Iterator<Item = ImportRecord> {
    records.into_iter().map(move |record| ImportRecord {
        is_type_only: type_only,
        ..record
    })
}

/// An `if TYPE_CHECKING:` statement, whose consequence block runs only under a
/// type checker. `typing.TYPE_CHECKING` and `TYPE_CHECKING and <version test>`
/// count; `not TYPE_CHECKING` does not, so its else-branch stays a runtime
/// import rather than being silently exempted.
fn guards_type_checking(node: Node<'_>, source: &[u8]) -> bool {
    node.kind() == "if_statement"
        && node
            .child_by_field_name("condition")
            .is_some_and(|condition| is_type_checking_test(condition, source))
}

fn is_type_checking_test(node: Node<'_>, source: &[u8]) -> bool {
    match node.kind() {
        "identifier" => node_text(node, source).as_deref() == Some("TYPE_CHECKING"),
        "attribute" => {
            node.child_by_field_name("attribute")
                .and_then(|attribute| node_text(attribute, source))
                .as_deref()
                == Some("TYPE_CHECKING")
        }
        "parenthesized_expression" => node
            .named_child(0)
            .is_some_and(|inner| is_type_checking_test(inner, source)),
        "boolean_operator" => {
            has_child_kind(node, "and")
                && ["left", "right"].iter().any(|field| {
                    node.child_by_field_name(field)
                        .is_some_and(|operand| is_type_checking_test(operand, source))
                })
        }
        _ => false,
    }
}

fn has_child_kind(node: Node<'_>, kind: &str) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|child| child.kind() == kind)
}

/// Names that `importlib` and `importlib.import_module` are bound to in this
/// file, so `il.import_module("x")` and a renamed `from importlib import
/// import_module as load` are both recognised while an unrelated
/// `self.import_module(...)` is not.
#[derive(Default)]
struct ImportlibBindings {
    modules: BTreeSet<String>,
    functions: BTreeSet<String>,
}

fn python_importlib_bindings(root: Node<'_>, source: &[u8]) -> ImportlibBindings {
    let mut bindings = ImportlibBindings::default();
    collect_importlib_bindings(root, source, &mut bindings);
    bindings
}

fn collect_importlib_bindings(node: Node<'_>, source: &[u8], bindings: &mut ImportlibBindings) {
    match node.kind() {
        "import_statement" => {
            for name in python_import_names(node) {
                if let Some(bound) = importlib_module_binding(name, source) {
                    bindings.modules.insert(bound);
                }
            }
        }
        "import_from_statement"
            if python_from_import_module(node, source).as_deref() == Some("importlib") =>
        {
            for name in python_import_names(node) {
                if python_import_target_text(name, source).as_deref() == Some("import_module") {
                    bindings.functions.insert(
                        python_import_alias(name, source).unwrap_or("import_module".to_string()),
                    );
                }
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_importlib_bindings(child, source, bindings);
    }
}

fn importlib_module_binding(name: Node<'_>, source: &[u8]) -> Option<String> {
    let target = python_import_target_text(name, source)?;
    if target == "importlib" {
        return Some(python_import_alias(name, source).unwrap_or(target));
    }
    // `import importlib.util` binds the top-level package name, not the submodule.
    if target.starts_with("importlib.") && python_import_alias(name, source).is_none() {
        return Some("importlib".to_string());
    }
    None
}

fn python_import_alias(name: Node<'_>, source: &[u8]) -> Option<String> {
    if name.kind() != "aliased_import" {
        return None;
    }
    node_text(name.child_by_field_name("alias")?, source)
}

fn python_from_import_module(node: Node<'_>, source: &[u8]) -> Option<String> {
    let module = node.child_by_field_name("module_name")?;
    if module.kind() == "relative_import" {
        return None;
    }
    node_text(module, source)
}

fn python_dynamic_import_record(
    node: Node<'_>,
    source: &[u8],
    importlib: &ImportlibBindings,
) -> Option<ImportRecord> {
    if !is_python_dynamic_import_call(node.child_by_field_name("function")?, source, importlib) {
        return None;
    }

    let arguments = node.child_by_field_name("arguments")?;
    let specifier = first_string_child(arguments)?;
    let record = python_record(
        python_dynamic_specifier(&node_text(specifier, source)?)?,
        specifier,
    );
    Some(ImportRecord {
        is_dynamic: true,
        ..record
    })
}

fn is_python_dynamic_import_call(
    function: Node<'_>,
    source: &[u8],
    importlib: &ImportlibBindings,
) -> bool {
    let Some(text) = node_text(function, source) else {
        return false;
    };
    match function.kind() {
        "identifier" => text == "__import__" || importlib.functions.contains(&text),
        "attribute" => function
            .child_by_field_name("object")
            .and_then(|object| node_text(object, source))
            .is_some_and(|object| {
                importlib.modules.contains(&object)
                    && function
                        .child_by_field_name("attribute")
                        .and_then(|attribute| node_text(attribute, source))
                        .as_deref()
                        == Some("import_module")
            }),
        _ => false,
    }
}

fn python_dynamic_specifier(raw: &str) -> Option<String> {
    let dots = raw.chars().take_while(|char| *char == '.').count();
    if dots == 0 {
        return (!raw.is_empty()).then(|| raw.to_string());
    }
    let module_path = raw[dots..].replace('.', "/");
    if module_path.is_empty() {
        return None;
    }
    Some(format!("{}{module_path}", python_dot_prefix(dots)))
}

fn python_import_records(node: Node<'_>, source: &[u8]) -> Vec<ImportRecord> {
    python_import_names(node)
        .into_iter()
        .filter_map(|name| {
            Some(python_record(
                python_import_target_text(name, source)?,
                name,
            ))
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
    Some(python_dot_prefix(dots))
}

fn python_dot_prefix(dots: usize) -> String {
    if dots <= 1 {
        "./".to_string()
    } else {
        "../".repeat(dots - 1)
    }
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
        scan_comments_from_tree(Path::new(path), tree.root_node(), content.as_bytes())
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
    fn extracts_python_dynamic_imports() {
        let records = scan_imports(
            &PathBuf::from("src/jobs/worker.py"),
            "import importlib\nfrom importlib import import_module as load\nfirst = importlib.import_module(\"requests\")\nsecond = load(\"app.lib.auth\")\nthird = __import__(\"json\")\nfourth = importlib.import_module(\".sibling\")\n",
        )
        .expect("python source should parse");

        assert_eq!(
            records
                .iter()
                .filter(|record| record.is_dynamic)
                .map(|record| (record.specifier.as_str(), record.line))
                .collect::<Vec<_>>(),
            vec![
                ("requests", 3),
                ("app.lib.auth", 4),
                ("json", 5),
                ("./sibling", 6),
            ]
        );
    }

    #[test]
    fn ignores_python_calls_that_are_not_bound_to_importlib() {
        let records = scan_imports(
            &PathBuf::from("src/jobs/worker.py"),
            "import_module(\"requests\")\nself.import_module(\"requests\")\nregistry.import_module(\"requests\")\n",
        )
        .expect("python source should parse");

        assert_eq!(records, vec![]);
    }

    #[test]
    fn marks_python_imports_guarded_by_type_checking() {
        let records = scan_imports(
            &PathBuf::from("src/jobs/worker.py"),
            "from typing import TYPE_CHECKING\n\nif TYPE_CHECKING:\n    from app.lib import shapes\n\nif typing.TYPE_CHECKING:\n    from app.lib import aliases\n\nif TYPE_CHECKING and sys.version_info >= (3, 11):\n    from app.lib import modern\n\nif TYPE_CHECKING:\n    if sys.platform == \"linux\":\n        from app.lib import nested\n",
        )
        .expect("python source should parse");

        assert_eq!(
            records
                .iter()
                .map(|record| (record.specifier.as_str(), record.is_type_only))
                .collect::<Vec<_>>(),
            vec![
                ("typing", false),
                ("app.lib", true),
                ("app.lib", true),
                ("app.lib", true),
                ("app.lib", true),
            ]
        );
    }

    #[test]
    fn keeps_runtime_branches_of_a_type_checking_guard_as_value_imports() {
        let records = scan_imports(
            &PathBuf::from("src/jobs/worker.py"),
            "if TYPE_CHECKING:\n    from app.lib import shapes\nelse:\n    from app.lib import fallback\n\nif TYPE_CHECKING:\n    from app.lib import other\nelif legacy:\n    from app.lib import old\n\nif not TYPE_CHECKING:\n    from app.lib import runtime\n\nif TYPE_CHECKING or debug:\n    from app.lib import loose\n",
        )
        .expect("python source should parse");

        assert_eq!(
            records
                .iter()
                .map(|record| record.is_type_only)
                .collect::<Vec<_>>(),
            vec![true, false, true, false, false, false]
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
    fn skips_swift_imports_inside_block_comments() {
        let records = scan_imports(
            &PathBuf::from("Sources/App/UI/ViewModel.swift"),
            "/*\nimport Core\n*/\n/* outer /* inner import Nested */ still */\nimport Foundation\n",
        )
        .expect("swift imports should scan");

        assert_eq!(
            records,
            vec![ImportRecord {
                specifier: "Foundation".to_string(),
                line: 5,
                is_type_only: false,
                is_dynamic: false,
            }]
        );
    }

    #[test]
    fn classifies_swift_block_and_triple_slash_comments() {
        assert_eq!(
            comments("View.swift", "/*\n * block\n */\n/// doc\n// plain\n"),
            vec![
                CommentRecord {
                    start_line: 1,
                    end_line: 3,
                    is_doc: false,
                },
                CommentRecord {
                    start_line: 4,
                    end_line: 4,
                    is_doc: true,
                },
                CommentRecord {
                    start_line: 5,
                    end_line: 5,
                    is_doc: false,
                },
            ]
        );
    }

    #[test]
    fn flattens_rust_use_trees_into_one_record_per_target() {
        let records = scan_imports(
            &PathBuf::from("crates/klint-rs/src/arch.rs"),
            "use crate::syntax::{TreeCache, scan_imports_from_tree};\nuse std::collections::{BTreeMap, BTreeSet};\nuse super::helper as alias;\nuse self::inner::*;\nuse crate::files::{self, normalize_path};\n// use crate::ignored::Thing;\n",
        )
        .expect("rust source should parse");

        assert_eq!(
            records
                .iter()
                .map(|record| (record.specifier.as_str(), record.line))
                .collect::<Vec<_>>(),
            vec![
                ("crate::syntax::TreeCache", 1),
                ("crate::syntax::scan_imports_from_tree", 1),
                ("std::collections::BTreeMap", 2),
                ("std::collections::BTreeSet", 2),
                ("super::helper", 3),
                ("self::inner", 4),
                ("crate::files", 5),
                ("crate::files::normalize_path", 5),
            ]
        );
    }

    #[test]
    fn classifies_rust_line_block_and_doc_comments() {
        assert_eq!(
            comments(
                "lib.rs",
                "//! inner\n/// doc\n// plain\n/*\n * block\n */\n"
            ),
            vec![
                CommentRecord {
                    start_line: 1,
                    end_line: 1,
                    is_doc: true,
                },
                CommentRecord {
                    start_line: 2,
                    end_line: 2,
                    is_doc: true,
                },
                CommentRecord {
                    start_line: 3,
                    end_line: 3,
                    is_doc: false,
                },
                CommentRecord {
                    start_line: 4,
                    end_line: 6,
                    is_doc: false,
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
