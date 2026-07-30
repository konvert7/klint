mod comments;
mod javascript;
mod jsx;
mod python;
mod rust;
mod swift;

use std::path::Path;
use tree_sitter::{Node, Parser};

use crate::syntax::{SourceLanguage, language_for_path, source_language_for_path};

pub(crate) use comments::scan_comments_from_tree;
use javascript::walk_imports;
pub(crate) use jsx::scan_jsx_elements_from_tree;
pub use jsx::{JsxElementRecord, scan_jsx_elements};
use python::scan_python_imports;
use rust::walk_rust_imports;
use swift::walk_swift_imports;

#[derive(Debug, PartialEq, Eq)]
pub struct ImportRecord {
    pub specifier: String,
    pub line: usize,
    pub is_type_only: bool,
    pub is_dynamic: bool,
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

fn first_string_child(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| child.kind() == "string")
}
