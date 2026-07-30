use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::{line_fix, rule_applies_to_file};
use crate::config::RuleConfig;
use crate::files::relative_path;
use crate::output::Violation;
use crate::syntax::{
    TreeCache, scan_prefer_at_from_tree, scan_prefer_nullish_coalescing_assign_from_tree,
    scan_prefer_string_raw_from_tree, scan_prefer_string_raw_regexp_from_tree,
    scan_prefer_string_replaceall_from_tree, scan_single_char_classes_from_tree,
};

pub(super) fn run_sonar_no_single_char_class(
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
        let records = scan_single_char_classes_from_tree(tree.root_node(), content.as_bytes());

        for record in records {
            violations.push(Violation {
                file: relative_path(root, file),
                line: record.line,
                rule: "sonar/no-single-char-class".to_string(),
                message: format!(
                    "Character class [{}] contains a single element — remove the brackets.",
                    record.class
                ),
                severity: severity.to_string(),
                fix: line_fix(
                    content,
                    record.start_row,
                    record.end_row,
                    record.start_byte,
                    record.end_byte,
                    &record.fixed_regex,
                ),
            });
        }
    }
    violations
}

pub(super) fn run_sonar_prefer_at(
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
        let records = scan_prefer_at_from_tree(tree.root_node(), content.as_bytes());

        for record in records {
            let replacement = format!("{}.at(-{})", record.base, record.offset);
            violations.push(Violation {
                file: relative_path(root, file),
                line: record.line,
                rule: "sonar/prefer-at".to_string(),
                message: format!(
                    "Prefer {} over {}[{}.length - {}] for cleaner negative indexing.",
                    replacement, record.base, record.base, record.offset
                ),
                severity: severity.to_string(),
                fix: line_fix(
                    content,
                    record.start_row,
                    record.end_row,
                    record.start_byte,
                    record.end_byte,
                    &replacement,
                ),
            });
        }
    }
    violations
}

pub(super) fn run_sonar_prefer_string_replaceall(
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
        let records = scan_prefer_string_replaceall_from_tree(tree.root_node(), content.as_bytes());

        for record in records {
            let replacement = format!(
                "{}.replaceAll({}, {})",
                record.receiver, record.pattern_lit, record.replacement
            );
            violations.push(Violation {
                file: relative_path(root, file),
                line: record.line,
                rule: "sonar/prefer-string-replaceall".to_string(),
                message: format!(
                    "Prefer `{}.replaceAll({}, ...)` over `.replace(/{}/g, ...)` — replaceAll() with a string is clearer and avoids regex escaping pitfalls.",
                    record.receiver, record.pattern_lit, record.pattern
                ),
                severity: severity.to_string(),
                fix: line_fix(
                    content,
                    record.start_row,
                    record.end_row,
                    record.start_byte,
                    record.end_byte,
                    &replacement,
                ),
            });
        }
    }
    violations
}

pub(super) fn run_sonar_prefer_string_raw_regexp(
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
        let records = scan_prefer_string_raw_regexp_from_tree(tree.root_node(), content.as_bytes());

        for record in records {
            violations.push(Violation {
                file: relative_path(root, file),
                line: record.line,
                rule: "sonar/prefer-string-raw-regexp".to_string(),
                message:
                    "Use String.raw`...` for RegExp template argument to avoid double backslashes (Sonar S7780)."
                        .to_string(),
                severity: severity.to_string(),
                fix: line_fix(
                    content,
                    record.start_row,
                    record.end_row,
                    record.start_byte,
                    record.end_byte,
                    &record.fixed_arg,
                ),
            });
        }
    }
    violations
}

pub(super) fn run_sonar_prefer_string_raw(
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
        let records = scan_prefer_string_raw_from_tree(tree.root_node(), content.as_bytes());

        for record in records {
            violations.push(Violation {
                file: relative_path(root, file),
                line: record.line,
                rule: "sonar/prefer-string-raw".to_string(),
                message:
                    "String literal with escaped backslashes — use String.raw`...` for clarity (Sonar S6535)."
                        .to_string(),
                severity: severity.to_string(),
                fix: line_fix(
                    content,
                    record.start_row,
                    record.end_row,
                    record.start_byte,
                    record.end_byte,
                    &record.fixed,
                ),
            });
        }
    }
    violations
}

pub(super) fn run_sonar_prefer_nullish_coalescing_assign(
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
        let records =
            scan_prefer_nullish_coalescing_assign_from_tree(tree.root_node(), content.as_bytes());

        for record in records {
            let replacement = format!("{} ??= {};", record.target, record.value);
            let message_replacement = format!("{} ??= {}", record.target, record.value);
            violations.push(Violation {
                file: relative_path(root, file),
                line: record.line,
                rule: "sonar/prefer-nullish-coalescing-assign".to_string(),
                message: format!(
                    "Prefer `{message_replacement}` over explicit nullish guard assignment — ??= only assigns when null or undefined."
                ),
                severity: severity.to_string(),
                fix: line_fix(
                    content,
                    record.start_row,
                    record.end_row,
                    record.start_byte,
                    record.end_byte,
                    &replacement,
                ),
            });
        }
    }
    violations
}
