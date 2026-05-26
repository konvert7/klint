use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::config::RuleConfig;
use crate::files::relative_path;
use crate::output::Violation;
use crate::syntax::{scan_nested_template_literals, scan_string_match};

pub(crate) fn run_supported_rules(
    rules: &BTreeMap<String, RuleConfig>,
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    root: &Path,
    violations: &mut Vec<Violation>,
) {
    if let Some(config) = rules.get("no-string-match") {
        run_no_string_match(config, files, file_contents, root, violations);
    }
    if let Some(config) = rules.get("no-nested-template-literals") {
        run_no_nested_template_literals(config, files, file_contents, root, violations);
    }
}

fn run_no_string_match(
    config: &RuleConfig,
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    root: &Path,
    violations: &mut Vec<Violation>,
) {
    let severity = config.severity();
    if severity == "off" {
        return;
    }

    for file in files {
        let Some(content) = file_contents.get(file) else {
            continue;
        };
        let Ok(records) = scan_string_match(file, content) else {
            continue;
        };

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
}

fn run_no_nested_template_literals(
    config: &RuleConfig,
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    root: &Path,
    violations: &mut Vec<Violation>,
) {
    let severity = config.severity();
    if severity == "off" {
        return;
    }

    for file in files {
        let Some(content) = file_contents.get(file) else {
            continue;
        };
        let Ok(records) = scan_nested_template_literals(file, content) else {
            continue;
        };

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
}

fn line_fix(
    content: &str,
    start_row: usize,
    end_row: usize,
    start_byte: usize,
    end_byte: usize,
    replacement: &str,
) -> Option<serde_json::Value> {
    if start_row != end_row {
        return None;
    }

    let line_start = line_start_byte(content, start_byte);
    let line_end = line_end_byte(content, start_byte);
    let mut line = content.get(line_start..line_end)?.to_string();
    line.replace_range(
        (start_byte - line_start)..(end_byte - line_start),
        replacement,
    );

    Some(serde_json::json!({
        "startLine": start_row + 1,
        "endLine": end_row + 1,
        "replacement": line,
    }))
}

fn line_start_byte(content: &str, byte: usize) -> usize {
    content[..byte].rfind('\n').map_or(0, |index| index + 1)
}

fn line_end_byte(content: &str, byte: usize) -> usize {
    content[byte..]
        .find('\n')
        .map_or(content.len(), |index| byte + index)
}
