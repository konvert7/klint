mod architecture;
mod rules;

pub use architecture::{
    CommentRecord, ImportRecord, JsxElementRecord, scan_imports, scan_jsx_elements,
};
pub(crate) use architecture::{
    scan_comments_from_tree, scan_csharp_namespaces, scan_imports_from_tree,
    scan_jsx_elements_from_tree,
};
pub use rules::{
    ConsecutiveArrayPushRecord, NestedTemplateLiteralRecord, PreferAtRecord,
    PreferNullishCoalescingAssignRecord, PreferStringRawRecord, PreferStringRawRegexpRecord,
    PreferStringReplaceAllRecord, SingleCharClassRecord, StringMatchRecord, SyncInAsyncRecord,
    UnguardedJsonParseRecord, scan_consecutive_array_push, scan_nested_template_literals,
    scan_prefer_at, scan_prefer_nullish_coalescing_assign, scan_prefer_string_raw,
    scan_prefer_string_raw_regexp, scan_prefer_string_replaceall, scan_single_char_classes,
    scan_string_match, scan_sync_in_async, scan_unguarded_json_parse,
};
pub(crate) use rules::{
    find_nested_template_literals, is_async_function_like, is_function_like, is_json_parse_call,
    prefer_at_record, prefer_nullish_coalescing_assign_record, prefer_string_raw_record,
    prefer_string_raw_regexp_record, prefer_string_replaceall_record, scan_statement_run,
    single_char_class_record, string_match_record, sync_call_name,
};

use std::path::Path;
use tree_sitter::{Language, Node, Parser, Tree};

/// Parses one file's source. The engine walks each file exactly once, so
/// there is no cache to miss — every call here is work that has to happen.
pub(crate) fn parse_source(path: &Path, content: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    parser.set_language(&language_for_path(path)).ok()?;
    parser.parse(content, None)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourceLanguage {
    JavaScriptLike,
    Python,
    Swift,
    Rust,
    CSharp,
}

fn language_for_path(path: &Path) -> Language {
    match source_language_for_path(path) {
        SourceLanguage::Python => tree_sitter_python::LANGUAGE.into(),
        SourceLanguage::Swift => tree_sitter_swift::LANGUAGE.into(),
        SourceLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
        SourceLanguage::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
        SourceLanguage::JavaScriptLike => {
            if is_jsx_path(path) {
                tree_sitter_typescript::LANGUAGE_TSX.into()
            } else {
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
            }
        }
    }
}

fn source_language_for_path(path: &Path) -> SourceLanguage {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("py") => SourceLanguage::Python,
        Some("swift") => SourceLanguage::Swift,
        Some("rs") => SourceLanguage::Rust,
        Some("cs") => SourceLanguage::CSharp,
        _ => SourceLanguage::JavaScriptLike,
    }
}

pub(crate) fn is_jsx_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("tsx" | "jsx")
    )
}

fn node_text(node: Node<'_>, source: &[u8]) -> Option<String> {
    let raw = raw_node_text(node, source)?;
    Some(raw.trim_matches(['"', '\'', '`']).to_string())
}

fn raw_node_text(node: Node<'_>, source: &[u8]) -> Option<String> {
    Some(node.utf8_text(source).ok()?.to_string())
}
