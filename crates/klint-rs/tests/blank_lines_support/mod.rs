#![allow(dead_code)]

use serde_json::Value;
use std::fs::{create_dir_all, read, remove_dir_all, write};
use std::path::{Path, PathBuf};
use std::process::Command;

pub mod edge_cases_py;
pub mod edge_cases_rs_sw_cs;
pub mod edge_cases_ts;
pub mod fix_cases;
pub mod fixtures;
mod fixtures_cs;
mod fixtures_swift_rs;
mod fixtures_ts_py;

pub fn ws(source: &str) -> String {
    source.replace("@@", "  ")
}

pub struct EdgeCase {
    pub name: &'static str,
    pub file: &'static str,
    pub input: &'static str,
    pub expected: Option<&'static str>,
}

pub const BEFORE: &str = "blank-line-before-control-flow";
pub const AFTER: &str = "blank-line-after-block";

pub struct Fixture {
    pub root: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = remove_dir_all(&self.root);
    }
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

pub fn config_for(rules: &str) -> String {
    format!("include: [\"src\"]\nrules:\n{rules}")
}

pub fn simple(rule: &str) -> String {
    config_for(&format!("  {rule}: error\n"))
}

pub fn both() -> String {
    config_for(&format!("  {BEFORE}: error\n  {AFTER}: error\n"))
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

pub fn fix_twice(config: &str, file: &str, input: &str, expected: &str) {
    let fx = fixture("blank-fix", config, &[(file, input)]);
    let first = fix(&fx);
    assert_eq!(text(&fx, file), expected, "{file}: {first}");
    let second = fix(&fx);
    assert_eq!(text(&fx, file), expected, "{file} second run");
    assert!(second.contains("applied 0 fix(es)"), "{file}: {second}");
}

pub fn source(fx: &Fixture, file: &str) -> Vec<u8> {
    read(fx.root.join("src").join(file)).expect("read source")
}

pub fn text(fx: &Fixture, file: &str) -> String {
    String::from_utf8(source(fx, file)).expect("utf8 source")
}

pub fn hits(json: &Value, rule: &str) -> Vec<(String, u64)> {
    let mut found: Vec<(String, u64)> = json["violations"]
        .as_array()
        .expect("violations array")
        .iter()
        .filter(|v| v["rule"] == rule)
        .map(|v| {
            (
                v["file"].as_str().expect("file").to_string(),
                v["line"].as_u64().expect("line"),
            )
        })
        .collect();
    found.sort();
    found
}

pub fn lines_of(json: &Value, rule: &str, file: &str) -> Vec<u64> {
    let wanted = format!("src/{file}");
    hits(json, rule)
        .into_iter()
        .filter(|(f, _)| *f == wanted)
        .map(|(_, line)| line)
        .collect()
}

pub fn messages(json: &Value, rule: &str) -> Vec<String> {
    json["violations"]
        .as_array()
        .expect("violations array")
        .iter()
        .filter(|v| v["rule"] == rule)
        .map(|v| v["message"].as_str().expect("message").to_string())
        .collect()
}

pub fn severities(json: &Value, rule: &str) -> Vec<String> {
    json["violations"]
        .as_array()
        .expect("violations array")
        .iter()
        .filter(|v| v["rule"] == rule)
        .map(|v| v["severity"].as_str().expect("severity").to_string())
        .collect()
}
