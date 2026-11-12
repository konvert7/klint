#![allow(dead_code)]

use crate::migrate_support::*;
use serde_json::Value;
use std::fs::write;

pub const DATES: &str =
    "const a = new Date(1);\nconst b = new Date(2);\nexport const same = a === b;\n";
pub const DROPPED: &str = "async function f() {}\nf();\n";
pub const BOTH: &str = "async function f() {}\nconst a = new Date(1);\nconst b = new Date(2);\nf();\nexport const same = a === b;\n";
pub const DATE_RULE: &str = "no-date-equality";
pub const PROMISE_RULE: &str = "no-floating-promise";

pub fn with_source(config: &str, source: &str) -> Project {
    let p = project("klint.yaml", config.as_bytes());
    write(p.root.join("src/a.ts"), source).expect("write source");
    p
}

pub fn findings_of(json: &Value, rule: &str) -> Vec<Value> {
    violations(json)
        .into_iter()
        .filter(|v| v["rule"] == rule)
        .collect()
}

pub fn rule_warnings(json: &Value) -> Vec<Value> {
    violations(json)
        .into_iter()
        .filter(|v| v["rule"] == "klint/config" || v["rule"] == "klint/unsupported-rule")
        .filter(|v| !v["message"].as_str().expect("m").contains("v1 format"))
        .collect()
}

pub fn warnings_naming(json: &Value, rule: &str) -> Vec<String> {
    rule_warnings(json)
        .iter()
        .map(|v| v["message"].as_str().expect("m").to_string())
        .filter(|m| m.contains(rule))
        .collect()
}

pub fn experimental(entries: &str) -> String {
    format!("include: [\"src\"]\nrules:\n  experimental:\n{entries}")
}

pub const DATE_CONTROL: &str =
    "const zzA = new Date(7);\nconst zzB = new Date(8);\nexport const zzSame = zzA === zzB;\n";
pub const PROMISE_CONTROL: &str = "async function zzRun() {}\nzzRun();\n";
pub const DATE_CONTROL_LINE: u64 = 3;
pub const PROMISE_CONTROL_LINE: u64 = 2;

pub fn findings_in(json: &Value, file: &str, rule: &str) -> Vec<Value> {
    findings_of(json, rule)
        .into_iter()
        .filter(|v| v["file"].as_str().is_some_and(|f| f.ends_with(file)))
        .collect()
}

pub fn lines(findings: &[Value]) -> Vec<u64> {
    findings
        .iter()
        .map(|v| v["line"].as_u64().expect("line"))
        .collect()
}

pub fn rule_run(rule: &str, control: &str, source: &str) -> (Vec<Value>, Vec<Value>) {
    let p = project(
        "klint.yaml",
        experimental(&format!("    {rule}: error\n")).as_bytes(),
    );
    write(p.root.join("src/a.ts"), source).expect("write a.ts");
    write(p.root.join("src/control.ts"), control).expect("write control.ts");
    let json = lint_json(&p.root);
    (
        findings_in(&json, "a.ts", rule),
        findings_in(&json, "control.ts", rule),
    )
}

pub fn expect_rule(rule: &str, control: &str, control_line: u64, source: &str, expected: &[u64]) {
    let (row, control_found) = rule_run(rule, control, source);
    assert_eq!(lines(&row), expected, "{source}");
    assert_eq!(lines(&control_found), [control_line], "control must report");
}

pub fn expect_date(source: &str, expected: &[u64]) {
    expect_rule(DATE_RULE, DATE_CONTROL, DATE_CONTROL_LINE, source, expected);
}

pub fn expect_promise(source: &str, expected: &[u64]) {
    expect_rule(
        PROMISE_RULE,
        PROMISE_CONTROL,
        PROMISE_CONTROL_LINE,
        source,
        expected,
    );
}

pub fn grouped(group: &str, entry: &str) -> String {
    format!("include: [\"src\"]\nrules:\n  {group}:\n    {entry}\n")
}

pub fn warnings_for(config: &str, rule: &str) -> Vec<String> {
    warnings_naming(&lint_json(&with_source(config, DATES).root), rule)
}

pub fn unknown_rule_warning(name: &str) -> String {
    format!("Unknown rule \"{name}\" is ignored — check its name.")
}

pub fn misgrouped_warning(name: &str, belongs: &str, written: &str) -> String {
    format!(
        "Rule \"{name}\" belongs in rules.{belongs}, not rules.{written}, so this entry is ignored."
    )
}

pub fn core_string_match_json(source: &str) -> Value {
    let config = "include: [\"src\"]\nrules:\n  core:\n    no-string-match: error\n  experimental:\n    no-date-equality: error\n";
    let p = project("klint.yaml", config.as_bytes());
    write(p.root.join("src/a.ts"), source).expect("write a.ts");
    write(p.root.join("src/control.ts"), DATE_CONTROL).expect("write control.ts");
    lint_json(&p.root)
}
