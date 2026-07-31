use super::layers::{in_prefixes, resolve_layer_mask, resolve_layer_prefixes};
use super::resolve::{
    AliasEntry, PythonContext, index_rust_crate_roots, index_swift_modules, load_path_aliases,
    resolve_import,
};
use super::*;
use crate::files::{
    is_csharp_source, is_javascript_like_source, is_python_source, is_rust_source, is_swift_source,
    relative_path, supports_import_scan,
};
use crate::output::Violation;
use crate::syntax::scan_imports_from_tree;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use tree_sitter::Node;

/// Project-wide module indexes, built once per run rather than once per rule.
pub(super) struct ResolveContext {
    aliases: Vec<AliasEntry>,
    python: PythonContext,
    swift_modules: BTreeMap<String, PathBuf>,
    rust_crate_roots: BTreeSet<PathBuf>,
}

impl ResolveContext {
    pub(super) fn build(root: &Path, files: &[PathBuf]) -> Self {
        Self {
            aliases: load_path_aliases(root),
            python: PythonContext::index(root, files),
            swift_modules: index_swift_modules(root, files),
            rust_crate_roots: index_rust_crate_roots(files),
        }
    }
}

pub(super) struct ImportPass {
    scope: Vec<bool>,
    allow_type_only: bool,
    deny_packages: Vec<String>,
    deny_prefixes: Option<Vec<PathBuf>>,
    allow_prefixes: Option<Vec<PathBuf>>,
    message: Option<String>,
    severity: String,
}

pub(super) fn plan_import_passes(
    arch: &ArchConfig,
    files: &[PathBuf],
    root: &Path,
) -> Vec<ImportPass> {
    let Some(rules) = &arch.imports else {
        return Vec::new();
    };

    rules
        .iter()
        .filter(|rule| rule.deny.is_some() || rule.allow.is_some() || rule.deny_packages.is_some())
        .map(|rule| ImportPass {
            scope: resolve_layer_mask(&rule.from, arch.layers.as_ref(), root, files),
            allow_type_only: rule.type_only.as_deref() == Some("allow"),
            deny_packages: rule
                .deny_packages
                .as_ref()
                .map(StringOrVec::items)
                .unwrap_or_default(),
            deny_prefixes: rule
                .deny
                .as_ref()
                .map(|deny| resolve_layer_prefixes(deny, arch.layers.as_ref(), root)),
            allow_prefixes: rule
                .allow
                .as_ref()
                .map(|allow| resolve_layer_prefixes(allow, arch.layers.as_ref(), root)),
            message: rule.message.clone(),
            severity: rule.severity.as_deref().unwrap_or("error").to_string(),
        })
        .collect()
}

impl ImportPass {
    pub(super) fn covers(&self, file_index: usize, file: &Path) -> bool {
        self.scope.get(file_index).copied().unwrap_or(false) && supports_import_scan(file)
    }

    pub(super) fn scan(
        &self,
        file: &Path,
        root: &Path,
        node: Node<'_>,
        source: &[u8],
        context: &ResolveContext,
    ) -> Vec<Violation> {
        let mut violations = Vec::new();
        for import in scan_imports_from_tree(file, node, source) {
            if self.allow_type_only && import.is_type_only {
                continue;
            }
            let Some(resolved) = resolve_import(
                file,
                root,
                &import.specifier,
                &context.aliases,
                &context.python,
                &context.swift_modules,
                &context.rust_crate_roots,
            ) else {
                if denies_package(file, &import.specifier, &self.deny_packages) {
                    violations.push(self.violation(
                        file,
                        root,
                        import.line,
                        "Import of a denied package",
                    ));
                }
                continue;
            };

            let message = if let Some(prefixes) = &self.deny_prefixes {
                if !in_prefixes(&resolved, prefixes) {
                    continue;
                }
                "Import crosses a denied boundary"
            } else if let Some(prefixes) = &self.allow_prefixes {
                if in_prefixes(&resolved, prefixes) {
                    continue;
                }
                "Import is not in the allowed list"
            } else {
                continue;
            };

            violations.push(self.violation(file, root, import.line, message));
        }
        violations
    }

    fn violation(&self, file: &Path, root: &Path, line: usize, fallback: &str) -> Violation {
        Violation {
            file: relative_path(root, file),
            line,
            rule: "arch/imports".to_string(),
            message: self.message.as_deref().unwrap_or(fallback).to_string(),
            severity: self.severity.clone(),
            fix: None,
        }
    }
}

fn denies_package(file: &Path, specifier: &str, entries: &[String]) -> bool {
    package_separator(file).is_some_and(|separator| matches_package(specifier, entries, separator))
}

/// What splits a package specifier into segments — `/` for npm specifiers,
/// `.` for Python modules, so `next` covers `next/headers` and `os` covers
/// `os.path`. Languages without package-level imports yield nothing.
fn package_separator(file: &Path) -> Option<&'static str> {
    if is_javascript_like_source(file) {
        Some("/")
    } else if is_python_source(file) || is_swift_source(file) || is_csharp_source(file) {
        Some(".")
    } else if is_rust_source(file) {
        Some("::")
    } else {
        None
    }
}

fn matches_package(specifier: &str, entries: &[String], separator: &str) -> bool {
    entries
        .iter()
        .any(|entry| specifier == entry || specifier.starts_with(&format!("{entry}{separator}")))
}

#[cfg(test)]
mod tests {
    use super::{matches_package, package_separator};
    use std::path::Path;

    #[test]
    fn matches_npm_packages_by_slash_segment() {
        let entries = vec!["next".to_string(), "node:fs".to_string()];
        assert!(matches_package("next", &entries, "/"));
        assert!(matches_package("next/headers", &entries, "/"));
        assert!(matches_package("node:fs/promises", &entries, "/"));
        assert!(!matches_package("nextra", &entries, "/"));
    }

    #[test]
    fn matches_python_modules_by_dot_segment() {
        let entries = vec!["os".to_string(), "google.cloud".to_string()];
        assert!(matches_package("os", &entries, "."));
        assert!(matches_package("os.path", &entries, "."));
        assert!(matches_package("google.cloud.storage", &entries, "."));
        assert!(!matches_package("oscrypto", &entries, "."));
        assert!(!matches_package("google.protobuf", &entries, "."));
    }

    #[test]
    fn separates_swift_specifiers_by_dot() {
        assert_eq!(
            package_separator(Path::new("Sources/App/Core/Auth.swift")),
            Some(".")
        );
        let entries = vec!["UIKit".to_string()];
        assert!(matches_package("UIKit", &entries, "."));
        assert!(!matches_package("UIKitten", &entries, "."));
    }
}
