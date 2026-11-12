#![allow(dead_code)]

use serde_json::Value;
use std::fs::{create_dir_all, read, remove_dir_all, write};
use std::path::{Path, PathBuf};
use std::process::Command;

pub mod cs_cases;
pub mod cs_kinds;
pub mod py_cases;
pub mod rs_cases;
pub mod swift_cases;
pub mod swift_kinds;
pub mod ts_cases;
pub mod ts_else_cases;
pub mod ts_names;

pub const RULE: &str = "cognitive-complexity";
pub const ANY_NAME: &str = "*";

pub struct Case {
    pub name: &'static str,
    pub file: &'static str,
    pub source: &'static str,
    pub expected: &'static [(&'static str, &'static str, u64)],
}

pub struct Fixture {
    pub root: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = remove_dir_all(&self.root);
    }
}

pub fn config_with(options: &str) -> String {
    format!("include: [\"src\"]\nrules:\n  {RULE}:\n{options}")
}

pub fn config_max(max: u64) -> String {
    config_with(&format!("    max: {max}\n"))
}

pub fn fixture(name: &str, config: &str, files: &[(&str, &str)]) -> Fixture {
    let root = crate::common::temp_root(name);
    create_dir_all(root.join("src")).expect("create fixture dirs");
    write(root.join("klint.yaml"), config).expect("write config");

    for (path, text) in files {
        let target = root.join("src").join(path);
        create_dir_all(target.parent().expect("parent dir")).expect("create source dir");
        write(target, text).expect("write source");
    }

    Fixture { root }
}

fn run(root: &Path, flag: &str) -> (String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_klint-rs"))
        .args(["--config", &root.to_string_lossy(), flag])
        .env("KLINT_NO_STATS", "1")
        .output()
        .expect("klint-rs runs");
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

pub fn lint(fx: &Fixture) -> Value {
    let (stdout, stderr) = run(&fx.root, "--json");
    serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("json: {e}: {stdout} {stderr}"))
}

pub fn fix(fx: &Fixture) -> String {
    run(&fx.root, "--fix").0
}

pub fn bytes(fx: &Fixture, file: &str) -> Vec<u8> {
    read(fx.root.join("src").join(file)).expect("read source")
}

pub fn violations<'a>(json: &'a Value, rule: &str) -> Vec<&'a Value> {
    json["violations"]
        .as_array()
        .expect("violations array")
        .iter()
        .filter(|v| v["rule"] == rule)
        .collect()
}

pub fn messages(json: &Value, rule: &str) -> Vec<String> {
    violations(json, rule)
        .iter()
        .map(|v| v["message"].as_str().expect("message").to_string())
        .collect()
}

pub fn files_hit(json: &Value) -> Vec<String> {
    violations(json, RULE)
        .iter()
        .map(|v| v["file"].as_str().expect("file").to_string())
        .collect()
}

pub fn function_name(message: &str) -> String {
    message.split('`').nth(1).unwrap_or("").to_string()
}

pub fn score_of(message: &str) -> u64 {
    message
        .split("cognitive complexity ")
        .nth(1)
        .and_then(|rest| rest.split(' ').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("no score in {message}"))
}

pub fn reported(json: &Value) -> Vec<(u64, String, u64)> {
    violations(json, RULE)
        .iter()
        .map(|v| {
            let message = v["message"].as_str().expect("message");
            (
                v["line"].as_u64().expect("line"),
                function_name(message),
                score_of(message),
            )
        })
        .collect()
}

pub fn line_of(source: &str, needle: &str) -> u64 {
    let index = source
        .lines()
        .position(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("needle {needle:?} not in source"));
    index as u64 + 1
}

pub fn expected_rows(case: &Case) -> Vec<(u64, String, u64)> {
    case.expected
        .iter()
        .map(|(needle, name, score)| (line_of(case.source, needle), name.to_string(), *score))
        .collect()
}

pub fn assert_case(case: &Case) {
    let fx = fixture("cc-case", &config_max(1), &[(case.file, case.source)]);
    let json = lint(&fx);
    let got = reported(&json);
    let want = expected_rows(case);
    assert_eq!(got.len(), want.len(), "{}: {got:?} vs {want:?}", case.name);

    for (got, want) in got.iter().zip(&want) {
        let name_ok = want.1 == ANY_NAME || got.1 == want.1;
        assert!(
            got.0 == want.0 && got.2 == want.2 && name_ok,
            "{}: got {got:?}, want {want:?} (all: {:?})",
            case.name,
            reported(&json)
        );
    }
}

pub fn assert_cases(cases: &[Case]) {
    for case in cases {
        assert_case(case);
    }
}

pub fn ifs(count: usize) -> String {
    "  if (a) {\n    g();\n  }\n".repeat(count)
}

pub fn ts_function_scoring(name: &str, score: usize) -> String {
    format!("function {name}(a) {{\n{}}}\n", ifs(score))
}
