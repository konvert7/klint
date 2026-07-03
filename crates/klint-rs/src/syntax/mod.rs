mod architecture;
mod rules;

pub use architecture::{ImportRecord, JsxElementRecord, scan_imports, scan_jsx_elements};
pub(crate) use architecture::{scan_imports_from_tree, scan_jsx_elements_from_tree};
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
    scan_consecutive_array_push_from_tree, scan_nested_template_literals_from_tree,
    scan_prefer_at_from_tree, scan_prefer_nullish_coalescing_assign_from_tree,
    scan_prefer_string_raw_from_tree, scan_prefer_string_raw_regexp_from_tree,
    scan_prefer_string_replaceall_from_tree, scan_single_char_classes_from_tree,
    scan_string_match_from_tree, scan_sync_in_async_from_tree, scan_unguarded_json_parse_from_tree,
};

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tree_sitter::{Language, Node, Parser, Tree};

/// Parses each file at most once per run and lets every rule/arch scan that
/// needs an AST for that file reuse the same parse instead of independently
/// re-parsing the same source. `Tree` clones are cheap (tree-sitter
/// refcounts the underlying tree), so misses are memoized and hits just
/// clone out of the cache. Backed by a `Mutex` (rather than `RefCell`) so
/// the same cache can be shared across the threads that run rule/arch
/// checks concurrently — `Tree` is `Send + Sync`, so this is sound.
#[derive(Default)]
pub(crate) struct TreeCache {
    cache: Mutex<BTreeMap<PathBuf, Tree>>,
}

impl TreeCache {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn get_or_parse(&self, file: &Path, content: &str) -> Option<Tree> {
        if let Some(tree) = self
            .cache
            .lock()
            .expect("tree cache lock poisoned")
            .get(file)
        {
            return Some(tree.clone());
        }
        let tree = parse_tree(file, content)?;
        self.cache
            .lock()
            .expect("tree cache lock poisoned")
            .insert(file.to_path_buf(), tree.clone());
        Some(tree)
    }
}

fn parse_tree(path: &Path, content: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    parser.set_language(&language_for_path(path)).ok()?;
    parser.parse(content, None)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourceLanguage {
    JavaScriptLike,
    Python,
    Swift,
}

fn language_for_path(path: &Path) -> Language {
    match source_language_for_path(path) {
        SourceLanguage::Python => tree_sitter_python::LANGUAGE.into(),
        SourceLanguage::Swift => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
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
