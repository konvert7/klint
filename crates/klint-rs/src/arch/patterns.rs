use super::*;
use crate::files::{normalize_path, relative_path};
use crate::output::Violation;
use crate::syntax::{TreeCache, is_jsx_path, scan_jsx_elements_from_tree};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

struct PatternScan<'a> {
    rule_name: &'a str,
    pattern: &'a str,
    message: &'a str,
    severity: &'a str,
}

struct ElementScan<'a> {
    rule_name: &'a str,
    message: &'a str,
    severity: &'a str,
}

pub(super) fn run_arch_forbidden_rules(
    arch: &ArchConfig,
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    tree_cache: &TreeCache,
    root: &Path,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let Some(rules) = &arch.forbidden else {
        return violations;
    };

    for rule in rules {
        let scoped_files = resolve_layer_files(&rule.in_scope, arch.layers.as_ref(), root, files);
        if let Some(tags) = &rule.jsx_element {
            scan_jsx_elements_for_targets(
                &scoped_files,
                tags,
                file_contents,
                tree_cache,
                root,
                ElementScan {
                    rule_name: "arch/forbidden",
                    message: &rule.message,
                    severity: rule.severity.as_deref().unwrap_or("error"),
                },
                &mut violations,
            );
            continue;
        }

        let Some(pattern) = &rule.pattern else {
            continue;
        };
        scan_lines_for_pattern(
            &scoped_files,
            file_contents,
            root,
            PatternScan {
                rule_name: "arch/forbidden",
                pattern,
                message: &rule.message,
                severity: rule.severity.as_deref().unwrap_or("error"),
            },
            &mut violations,
        );
    }
    violations
}

pub(super) fn run_arch_singleton_rules(
    arch: &ArchConfig,
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    tree_cache: &TreeCache,
    root: &Path,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let Some(rules) = &arch.singleton else {
        return violations;
    };

    for rule in rules {
        let only_file = normalize_path(&root.join(&rule.only));
        let scoped_files = match &rule.in_scope {
            Some(scope) => resolve_layer_files(scope, arch.layers.as_ref(), root, files),
            None => files.to_vec(),
        };
        let checked_files = scoped_files
            .into_iter()
            .filter(|file| file != &only_file)
            .collect::<Vec<_>>();

        if let Some(tags) = &rule.jsx_element {
            scan_jsx_elements_for_targets(
                &checked_files,
                tags,
                file_contents,
                tree_cache,
                root,
                ElementScan {
                    rule_name: "arch/singleton",
                    message: &rule.message,
                    severity: rule.severity.as_deref().unwrap_or("error"),
                },
                &mut violations,
            );
            continue;
        }

        let Some(pattern) = &rule.pattern else {
            continue;
        };
        scan_lines_for_pattern(
            &checked_files,
            file_contents,
            root,
            PatternScan {
                rule_name: "arch/singleton",
                pattern,
                message: &rule.message,
                severity: rule.severity.as_deref().unwrap_or("error"),
            },
            &mut violations,
        );
    }
    violations
}

fn scan_jsx_elements_for_targets(
    files: &[PathBuf],
    targets: &StringOrVec,
    file_contents: &BTreeMap<PathBuf, String>,
    tree_cache: &TreeCache,
    root: &Path,
    scan: ElementScan<'_>,
    violations: &mut Vec<Violation>,
) {
    let target_names = targets.items();
    for file in files {
        // Only jsx-path files can ever contain a jsx node, so skip the rest
        // rather than asking the cache to parse them for nothing.
        if !is_jsx_path(file) {
            continue;
        }
        let Some(content) = file_contents.get(file) else {
            continue;
        };
        let Some(tree) = tree_cache.get_or_parse(file, content) else {
            continue;
        };
        let elements = scan_jsx_elements_from_tree(tree.root_node(), content.as_bytes());

        for element in elements {
            if !target_names.contains(&element.tag_name) {
                continue;
            }
            violations.push(Violation {
                file: relative_path(root, file),
                line: element.line,
                rule: scan.rule_name.to_string(),
                message: scan.message.to_string(),
                severity: scan.severity.to_string(),
                fix: None,
            });
        }
    }
}

const REGEX_PREFIX: &str = "re:";

enum LineMatcher {
    Literal(String),
    Regex(regex::Regex),
}

impl LineMatcher {
    fn build(pattern: &str) -> Self {
        let Some(source) = pattern.strip_prefix(REGEX_PREFIX) else {
            return Self::Literal(pattern.to_string());
        };
        match regex::Regex::new(source) {
            Ok(regex) => Self::Regex(regex),
            Err(error) => {
                eprintln!("klint: invalid regex in arch pattern {pattern:?}: {error}");
                Self::Regex(regex::Regex::new("[^\\s\\S]").expect("never-matching regex is valid"))
            }
        }
    }

    fn is_match(&self, line: &str) -> bool {
        match self {
            Self::Literal(pattern) => line.contains(pattern.as_str()),
            Self::Regex(regex) => regex.is_match(line),
        }
    }
}

fn scan_lines_for_pattern(
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    root: &Path,
    scan: PatternScan<'_>,
    violations: &mut Vec<Violation>,
) {
    let matcher = LineMatcher::build(scan.pattern);
    for file in files {
        let Some(content) = file_contents.get(file) else {
            continue;
        };

        for (index, line) in content.lines().enumerate() {
            if matcher.is_match(line) {
                violations.push(Violation {
                    file: relative_path(root, file),
                    line: index + 1,
                    rule: scan.rule_name.to_string(),
                    message: scan.message.to_string(),
                    severity: scan.severity.to_string(),
                    fix: None,
                });
            }
        }
    }
}
