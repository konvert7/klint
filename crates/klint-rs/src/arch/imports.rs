use super::layers::{in_prefixes, resolve_layer_prefixes};
use super::resolve::{
    PythonContext, index_rust_crate_roots, index_swift_modules, load_path_aliases, resolve_import,
};
use super::*;
use crate::files::{
    is_javascript_like_source, is_python_source, is_rust_source, is_swift_source, relative_path,
    supports_import_scan,
};
use crate::output::Violation;
use crate::syntax::{TreeCache, scan_imports_from_tree};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub(super) fn run_arch_import_rules(
    arch: &ArchConfig,
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    tree_cache: &TreeCache,
    root: &Path,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let Some(rules) = &arch.imports else {
        return violations;
    };
    let aliases = load_path_aliases(root);
    let python = PythonContext::index(root, files);
    let swift_modules = index_swift_modules(root, files);
    let rust_crate_roots = index_rust_crate_roots(files);

    for rule in rules {
        if rule.deny.is_none() && rule.allow.is_none() && rule.deny_packages.is_none() {
            continue;
        }
        let allow_type_only = rule.type_only.as_deref() == Some("allow");
        let deny_packages = rule
            .deny_packages
            .as_ref()
            .map(StringOrVec::items)
            .unwrap_or_default();

        let severity = rule.severity.as_deref().unwrap_or("error");
        let from_files = resolve_layer_files(&rule.from, arch.layers.as_ref(), root, files);
        let deny_prefixes = rule
            .deny
            .as_ref()
            .map(|deny| resolve_layer_prefixes(deny, arch.layers.as_ref(), root));
        let allow_prefixes = rule
            .allow
            .as_ref()
            .map(|allow| resolve_layer_prefixes(allow, arch.layers.as_ref(), root));

        for file in from_files {
            if !supports_import_scan(&file) {
                continue;
            }
            let Some(content) = file_contents.get(&file) else {
                continue;
            };
            let Some(tree) = tree_cache.get_or_parse(&file, content) else {
                continue;
            };
            let imports = scan_imports_from_tree(&file, tree.root_node(), content.as_bytes());

            for import in imports {
                if allow_type_only && import.is_type_only {
                    continue;
                }
                let Some(resolved) = resolve_import(
                    &file,
                    root,
                    &import.specifier,
                    &aliases,
                    &python,
                    &swift_modules,
                    &rust_crate_roots,
                ) else {
                    if denies_package(&file, &import.specifier, &deny_packages) {
                        violations.push(Violation {
                            file: relative_path(root, &file),
                            line: import.line,
                            rule: "arch/imports".to_string(),
                            message: rule
                                .message
                                .as_deref()
                                .unwrap_or("Import of a denied package")
                                .to_string(),
                            severity: severity.to_string(),
                            fix: None,
                        });
                    }
                    continue;
                };

                let message = if let Some(prefixes) = &deny_prefixes {
                    if !in_prefixes(&resolved, prefixes) {
                        continue;
                    }
                    rule.message
                        .as_deref()
                        .unwrap_or("Import crosses a denied boundary")
                } else if let Some(prefixes) = &allow_prefixes {
                    if in_prefixes(&resolved, prefixes) {
                        continue;
                    }
                    rule.message
                        .as_deref()
                        .unwrap_or("Import is not in the allowed list")
                } else {
                    continue;
                };

                violations.push(Violation {
                    file: relative_path(root, &file),
                    line: import.line,
                    rule: "arch/imports".to_string(),
                    message: message.to_string(),
                    severity: severity.to_string(),
                    fix: None,
                });
            }
        }
    }
    violations
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
    } else if is_python_source(file) || is_swift_source(file) {
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
