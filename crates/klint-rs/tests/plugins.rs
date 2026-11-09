mod common;

use common::temp_root;
use serde_json::Value;
use std::fs::{create_dir_all, remove_dir_all, write};
use std::path::Path;
use std::process::Command;

const EVERY_SONAR_VIOLATION: &str = r#"declare const arr: number[], text: string, repl: string;
let x: object | undefined;
const a = arr[arr.length - 1];
if (x == null) x = {};
const p = "C:\\Users\\Documents\\file.txt";
const r = new RegExp(`\\.foo`);
const t = text.replace(/foo/g, repl);
const c = /a[b]c/;
"#;

fn write_fixture(root: &Path, config: &str) {
    create_dir_all(root.join("src")).expect("create fixture dirs");
    write(root.join("klint.yaml"), config).expect("write config");
    write(root.join("src/index.ts"), EVERY_SONAR_VIOLATION).expect("write source");
}

fn run_json(root: &Path) -> (i32, Value, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_klint-rs"))
        .args(["--config", &root.to_string_lossy(), "--json"])
        .env("KLINT_NO_STATS", "1")
        .output()
        .expect("klint-rs binary should execute");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    (
        output.status.code().expect("process should exit normally"),
        serde_json::from_str(&stdout).unwrap_or(Value::Null),
        String::from_utf8(output.stderr).expect("stderr should be utf8"),
    )
}

fn rule_severities(json: &Value) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = json["violations"]
        .as_array()
        .expect("violations array")
        .iter()
        .map(|v| {
            (
                v["rule"].as_str().expect("rule").to_string(),
                v["severity"].as_str().expect("severity").to_string(),
            )
        })
        .collect();
    found.sort();
    found
}

fn pairs(expected: &[(&str, &str)]) -> Vec<(String, String)> {
    expected
        .iter()
        .map(|(rule, severity)| (rule.to_string(), severity.to_string()))
        .collect()
}

#[test]
fn sonar_plugin_enables_all_six_rules_as_errors() {
    let root = temp_root("plugin-sonar");
    write_fixture(&root, "include: [\"src\"]\nplugins: [sonar]\n");

    let (status, json, stderr) = run_json(&root);

    assert_eq!(status, 2, "{stderr}");
    assert_eq!(
        rule_severities(&json),
        pairs(&[
            ("sonar/no-single-char-class", "error"),
            ("sonar/prefer-at", "error"),
            ("sonar/prefer-nullish-coalescing-assign", "error"),
            ("sonar/prefer-string-raw", "error"),
            ("sonar/prefer-string-raw-regexp", "error"),
            ("sonar/prefer-string-replaceall", "error"),
        ])
    );
    let _ = remove_dir_all(root);
}

#[test]
fn explicit_rules_override_plugin_defaults() {
    let root = temp_root("plugin-override");
    write_fixture(
        &root,
        "include: [\"src\"]\nplugins: [sonar]\nrules:\n  sonar/prefer-at: warn\n  sonar/prefer-string-raw: off\n  sonar/no-single-char-class: off\n  sonar/prefer-nullish-coalescing-assign: off\n  sonar/prefer-string-raw-regexp: off\n  sonar/prefer-string-replaceall: off\n",
    );

    let (status, json, stderr) = run_json(&root);

    assert_eq!(status, 0, "{stderr}");
    assert_eq!(
        rule_severities(&json),
        pairs(&[("sonar/prefer-at", "warn")])
    );
    let _ = remove_dir_all(root);
}

#[test]
fn unknown_plugin_fails_with_its_name() {
    let root = temp_root("plugin-unknown");
    write_fixture(&root, "include: [\"src\"]\nplugins: [nope]\n");

    let (status, _json, stderr) = run_json(&root);

    assert_eq!(status, 1, "{stderr}");
    assert!(
        stderr.contains("Unknown klint plugin: \"nope\""),
        "{stderr}"
    );
    let _ = remove_dir_all(root);
}
