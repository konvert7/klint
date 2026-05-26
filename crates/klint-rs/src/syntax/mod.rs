mod architecture;
mod rules;

pub use architecture::{ImportRecord, JsxElementRecord, scan_imports, scan_jsx_elements};
pub use rules::{
    ConsecutiveArrayPushRecord, NestedTemplateLiteralRecord, PreferAtRecord,
    PreferNullishCoalescingAssignRecord, PreferStringRawRecord, PreferStringRawRegexpRecord,
    PreferStringReplaceAllRecord, SingleCharClassRecord, StringMatchRecord, SyncInAsyncRecord,
    UnguardedJsonParseRecord, scan_consecutive_array_push, scan_nested_template_literals,
    scan_prefer_at, scan_prefer_nullish_coalescing_assign, scan_prefer_string_raw,
    scan_prefer_string_raw_regexp, scan_prefer_string_replaceall, scan_single_char_classes,
    scan_string_match, scan_sync_in_async, scan_unguarded_json_parse,
};

use std::path::Path;
use tree_sitter::{Language, Node};

fn language_for_path(path: &Path) -> Language {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("tsx" | "jsx") => tree_sitter_typescript::LANGUAGE_TSX.into(),
        _ => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
    }
}

fn is_jsx_path(path: &Path) -> bool {
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
