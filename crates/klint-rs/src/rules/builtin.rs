use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::{line_fix, rule_applies_to_file};
use crate::config::RuleConfig;
use crate::files::relative_path;
use crate::output::Violation;
use crate::syntax::{
    TreeCache, scan_consecutive_array_push_from_tree, scan_nested_template_literals_from_tree,
    scan_string_match_from_tree, scan_sync_in_async_from_tree, scan_unguarded_json_parse_from_tree,
};

pub(super) fn run_no_string_match(
    config: &RuleConfig,
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    tree_cache: &TreeCache,
    root: &Path,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let severity = config.severity();
    if severity == "off" {
        return violations;
    }

    for file in files {
        if !rule_applies_to_file(config, root, file) {
            continue;
        }
        let Some(content) = file_contents.get(file) else {
            continue;
        };
        let Some(tree) = tree_cache.get_or_parse(file, content) else {
            continue;
        };
        let records = scan_string_match_from_tree(tree.root_node(), content.as_bytes());

        for record in records {
            let replacement = format!("new RegExp({}).exec({})", record.regex, record.receiver);
            violations.push(Violation {
                file: relative_path(root, file),
                line: record.line,
                rule: "no-string-match".to_string(),
                message: format!(
                    "Use RegExp.exec() instead of String.match() for non-global regexes — use {replacement} instead."
                ),
                severity: severity.to_string(),
                fix: line_fix(content, record.start_row, record.end_row, record.start_byte, record.end_byte, &replacement),
            });
        }
    }
    violations
}

pub(super) fn run_no_nested_template_literals(
    config: &RuleConfig,
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    tree_cache: &TreeCache,
    root: &Path,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let severity = config.severity();
    if severity == "off" {
        return violations;
    }

    for file in files {
        if !rule_applies_to_file(config, root, file) {
            continue;
        }
        let Some(content) = file_contents.get(file) else {
            continue;
        };
        let Some(tree) = tree_cache.get_or_parse(file, content) else {
            continue;
        };
        let records = scan_nested_template_literals_from_tree(tree.root_node());

        for record in records {
            violations.push(Violation {
                file: relative_path(root, file),
                line: record.line,
                rule: "no-nested-template-literals".to_string(),
                message:
                    "Nested template literal — extract the inner template to a variable to improve readability."
                        .to_string(),
                severity: severity.to_string(),
                fix: None,
            });
        }
    }
    violations
}

pub(super) fn run_no_consecutive_array_push(
    config: &RuleConfig,
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    tree_cache: &TreeCache,
    root: &Path,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let severity = config.severity();
    if severity == "off" {
        return violations;
    }

    for file in files {
        if !rule_applies_to_file(config, root, file) {
            continue;
        }
        let Some(content) = file_contents.get(file) else {
            continue;
        };
        let Some(tree) = tree_cache.get_or_parse(file, content) else {
            continue;
        };
        let records = scan_consecutive_array_push_from_tree(tree.root_node(), content.as_bytes());

        for record in records {
            violations.push(Violation {
                file: relative_path(root, file),
                line: record.line,
                rule: "no-consecutive-array-push".to_string(),
                message: format!(
                    "{} consecutive .push() calls on `{}` — combine into a single .push(a, b, …) call.",
                    record.count, record.receiver
                ),
                severity: severity.to_string(),
                fix: None,
            });
        }
    }
    violations
}

pub(super) fn run_no_unguarded_json_parse(
    config: &RuleConfig,
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    tree_cache: &TreeCache,
    root: &Path,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let severity = config.severity();
    if severity == "off" {
        return violations;
    }

    for file in files {
        if !rule_applies_to_file(config, root, file) {
            continue;
        }
        let Some(content) = file_contents.get(file) else {
            continue;
        };
        let Some(tree) = tree_cache.get_or_parse(file, content) else {
            continue;
        };
        let records = scan_unguarded_json_parse_from_tree(tree.root_node(), content.as_bytes());

        for record in records {
            violations.push(Violation {
                file: relative_path(root, file),
                line: record.line,
                rule: "no-unguarded-json-parse".to_string(),
                message:
                    "JSON.parse() called without a surrounding try/catch — a malformed payload will throw an unhandled exception."
                        .to_string(),
                severity: severity.to_string(),
                fix: None,
            });
        }
    }
    violations
}

pub(super) fn run_no_sync_in_async(
    config: &RuleConfig,
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    tree_cache: &TreeCache,
    root: &Path,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let severity = config.severity();
    if severity == "off" {
        return violations;
    }

    for file in files {
        if !rule_applies_to_file(config, root, file) {
            continue;
        }
        let Some(content) = file_contents.get(file) else {
            continue;
        };
        let Some(tree) = tree_cache.get_or_parse(file, content) else {
            continue;
        };
        let records = scan_sync_in_async_from_tree(tree.root_node(), content.as_bytes());

        for record in records {
            violations.push(Violation {
                file: relative_path(root, file),
                line: record.line,
                rule: "no-sync-in-async".to_string(),
                message: format!(
                    "{}() blocks the event loop inside an async function — use the async equivalent from node:fs/promises.",
                    record.name
                ),
                severity: severity.to_string(),
                fix: None,
            });
        }
    }
    violations
}
