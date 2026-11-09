mod common;

use common::temp_root;
use serde_json::Value;
use std::fs::{create_dir_all, remove_dir_all, write};
use std::path::Path;
use std::process::Command;

fn run_json(root: &Path) -> (i32, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_klint-rs"))
        .args(["--config", &root.to_string_lossy(), "--json"])
        .env("KLINT_NO_STATS", "1")
        .output()
        .expect("klint-rs binary should execute");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    (
        output.status.code().expect("process should exit normally"),
        serde_json::from_str(&stdout).expect("stdout should be JSON"),
    )
}

fn fixture(name: &str, config: &str) -> std::path::PathBuf {
    let root = temp_root(name);
    create_dir_all(root.join("src")).expect("create fixture dirs");
    write(root.join("klint.yaml"), config).expect("write config");
    write(root.join("src/index.ts"), "export const a = 1;\n").expect("write source");
    root
}

fn advisories(json: &Value) -> Vec<(String, u64, String)> {
    json["violations"]
        .as_array()
        .expect("violations array")
        .iter()
        .filter(|v| v["rule"] == "klint/unsupported-rule")
        .map(|v| {
            assert_eq!(v["severity"], "warn");
            (
                v["file"].as_str().expect("file").to_string(),
                v["line"].as_u64().expect("line"),
                v["message"].as_str().expect("message").to_string(),
            )
        })
        .collect()
}

#[test]
fn warns_about_each_enabled_rule_the_rust_engine_cannot_run() {
    let root = fixture(
        "unsupported",
        "include: [\"src\"]\nrules:\n  no-floating-promise: error\n  no-date-equality: warn\n  no-misused-promises: off\n  no-such-rule: error\n  sonar/prefer-at: error\n",
    );

    let (status, json) = run_json(&root);

    assert_eq!(status, 0);
    assert_eq!(
        advisories(&json),
        vec![
            (
                "klint.yaml".to_string(),
                4,
                "no-date-equality needs a TypeScript type checker, which klint's Rust engine does not have, so it is ignored.".to_string()
            ),
            (
                "klint.yaml".to_string(),
                3,
                "no-floating-promise needs a TypeScript type checker, which klint's Rust engine does not have, so it is ignored — Biome's nursery/noFloatingPromises covers it.".to_string()
            ),
            (
                "klint.yaml".to_string(),
                6,
                "Unknown rule \"no-such-rule\" is ignored — check its name.".to_string()
            ),
        ]
    );
    let _ = remove_dir_all(root);
}

#[test]
fn warns_that_custom_rules_in_klint_rules_ts_are_ignored() {
    let root = fixture("custom-rules", "include: [\"src\"]\nrules: {}\n");
    write(root.join("klint.rules.ts"), "export default {};\n").expect("write custom rules");

    let (_, json) = run_json(&root);

    assert_eq!(
        advisories(&json),
        vec![(
            "klint.rules.ts".to_string(),
            1,
            "Custom rules in klint.rules.ts are not supported by klint's Rust engine and are ignored.".to_string()
        )]
    );
    let _ = remove_dir_all(root);
}

#[test]
fn stays_quiet_when_every_enabled_rule_is_supported() {
    let root = fixture(
        "supported",
        "include: [\"src\"]\nplugins: [sonar]\nrules:\n  no-async-predicate: error\n  no-floating-promise: off\n",
    );

    let (_, json) = run_json(&root);

    assert!(advisories(&json).is_empty(), "{json}");
    let _ = remove_dir_all(root);
}
