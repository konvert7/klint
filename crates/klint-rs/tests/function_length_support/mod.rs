#![allow(dead_code)]

use crate::complexity_support::{fixture, lint, messages, violations};
use serde_json::Value;

pub const RULE: &str = "function-length";

pub fn config(options: &str) -> String {
    format!("include: [\"src\"]\nrules:\n  {RULE}:\n{options}")
}

pub fn config_max_one(skip_blank_lines: bool) -> String {
    config(&format!(
        "    max: 1\n    skipBlankLines: {skip_blank_lines}\n"
    ))
}

pub fn count_of(message: &str) -> u64 {
    message
        .split(" has ")
        .nth(1)
        .and_then(|rest| rest.split(' ').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("no count in {message}"))
}

pub fn name_of(message: &str) -> String {
    message.split('`').nth(1).unwrap_or("").to_string()
}

pub fn rows(json: &Value) -> Vec<(u64, String, u64)> {
    violations(json, RULE)
        .iter()
        .map(|v| {
            let message = v["message"].as_str().expect("message");
            (
                v["line"].as_u64().expect("line"),
                name_of(message),
                count_of(message),
            )
        })
        .collect()
}

pub fn rows_for(file: &str, source: &str, skip_blank_lines: bool) -> Vec<(u64, String, u64)> {
    let fx = fixture("fl", &config_max_one(skip_blank_lines), &[(file, source)]);
    rows(&lint(&fx))
}

pub fn counts_for(file: &str, source: &str, skip_blank_lines: bool) -> Vec<u64> {
    rows_for(file, source, skip_blank_lines)
        .into_iter()
        .map(|r| r.2)
        .collect()
}

pub fn expect_count(file: &str, source: &str, skip_blank_lines: bool, want: u64) {
    let got = counts_for(file, source, skip_blank_lines);
    assert_eq!(
        got,
        vec![want],
        "{file} skip={skip_blank_lines}: {source:?}"
    );
}

pub fn expect_not_reported(file: &str, source: &str) {
    let got = counts_for(file, source, false);
    assert!(got.is_empty(), "{file}: {source:?} reported {got:?}");
}

pub fn max_message(name: &str, count: u64, max: u64) -> String {
    format!("Function `{name}` has {count} lines (max {max}) — split it into smaller functions.")
}

pub fn ts_function_of_lines(name: &str, body_lines: usize) -> String {
    format!(
        "function {name}() {{\n{}}}\n",
        "  a();\n".repeat(body_lines)
    )
}

pub fn lint_messages(options: &str, files: &[(&str, &str)]) -> Vec<String> {
    let fx = fixture("fl-msg", &config(options), files);
    messages(&lint(&fx), RULE)
}

pub fn files_hit_for(json: &Value, rule: &str) -> Vec<String> {
    violations(json, rule)
        .iter()
        .map(|v| v["file"].as_str().expect("file").to_string())
        .collect()
}
