mod common;

use common::temp_root;
use serde_json::Value;
use std::fs::{create_dir_all, remove_dir_all, write};
use std::process::Command;

const PREFER_AT: &str = "const x = arr[arr.length - 1];\n";
const V1_WARNING: &str =
    "This config uses the v1 format — run `klint migrate` to upgrade it in place.";

fn run_json(config_file: &str, config: &str) -> (i32, Value) {
    let root = temp_root("config-warnings");
    create_dir_all(root.join("src")).expect("create fixture dirs");
    write(root.join(config_file), config).expect("write config");
    write(root.join("src/a.ts"), PREFER_AT).expect("write source");
    let output = Command::new(env!("CARGO_BIN_EXE_klint-rs"))
        .args(["--config", &root.to_string_lossy(), "--json"])
        .env("KLINT_NO_STATS", "1")
        .output()
        .expect("klint-rs runs");
    let _ = remove_dir_all(&root);
    (
        output.status.code().expect("exits normally"),
        serde_json::from_slice(&output.stdout).expect("stdout is JSON"),
    )
}

fn violations_of(json: &Value, rule: &str) -> Vec<(String, u64, String, String)> {
    json["violations"]
        .as_array()
        .expect("violations array")
        .iter()
        .filter(|v| v["rule"] == rule)
        .map(|v| {
            (
                v["file"].as_str().expect("file").to_string(),
                v["line"].as_u64().expect("line"),
                v["severity"].as_str().expect("severity").to_string(),
                v["message"].as_str().expect("message").to_string(),
            )
        })
        .collect()
}

#[test]
fn warns_about_an_unknown_top_level_key_and_names_its_line() {
    let (status, json) = run_json(
        "klint.yaml",
        "include: [\"src\"]\nrulez:\n  sonar/prefer-at: error\nrules: {}\n",
    );

    assert_eq!(status, 0);
    assert_eq!(
        violations_of(&json, "klint/config"),
        vec![(
            "klint.yaml".to_string(),
            2,
            "warn".to_string(),
            "Unknown config key \"rulez\" is ignored — check its name.".to_string()
        )]
    );
}

#[test]
fn warns_about_unknown_keys_in_a_json_config_too() {
    let (_, json) = run_json(
        "klint.config.json",
        "{\n  \"include\": [\"src\"],\n  \"plugin\": [\"sonar\"]\n}\n",
    );

    assert_eq!(
        violations_of(&json, "klint/config"),
        vec![(
            "klint.config.json".to_string(),
            3,
            "warn".to_string(),
            "Unknown config key \"plugin\" is ignored — check its name.".to_string()
        )]
    );
}

#[test]
fn skips_a_rule_with_an_invalid_severity_and_says_so() {
    let (status, json) = run_json(
        "klint.yaml",
        "include: [\"src\"]\nrules:\n  sonar/prefer-at: fatal\n",
    );

    assert_eq!(status, 0);
    assert!(violations_of(&json, "sonar/prefer-at").is_empty(), "{json}");
    assert_eq!(
        violations_of(&json, "klint/config"),
        vec![(
            "klint.yaml".to_string(),
            3,
            "warn".to_string(),
            "Rule \"sonar/prefer-at\" has severity \"fatal\", which is not error, warn or off, so it is ignored.".to_string()
        ),
        (
            "klint.yaml".to_string(),
            1,
            "warn".to_string(),
            V1_WARNING.to_string()
        )]
    );
}

#[test]
fn checks_the_severity_inside_rule_options_too() {
    let (_, json) = run_json(
        "klint.yaml",
        "include: [\"src\"]\nrules:\n  sonar/prefer-at:\n    severity: warning\n",
    );

    assert!(violations_of(&json, "sonar/prefer-at").is_empty(), "{json}");
    assert_eq!(violations_of(&json, "klint/config").len(), 2, "{json}");
}

#[test]
fn stays_quiet_for_a_valid_config() {
    let (_, json) = run_json(
        "klint.yaml",
        "$schema: ./klint.schema.json\nroot: .\ninclude: [\"src\"]\nplugins: [sonar]\nrules:\n  sonar/prefer-at: warn\narch: {}\n",
    );

    assert_eq!(
        violations_of(&json, "klint/config"),
        vec![(
            "klint.yaml".to_string(),
            1,
            "warn".to_string(),
            V1_WARNING.to_string()
        )],
        "{json}"
    );
    assert_eq!(violations_of(&json, "sonar/prefer-at").len(), 1, "{json}");
}
