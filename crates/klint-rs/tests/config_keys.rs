mod common;
mod migrate_support;

use migrate_support::*;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fs::{read_to_string, write};
use std::path::PathBuf;
use std::process::Command;

fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/config-keys.json")
}

fn generated() -> BTreeSet<String> {
    let output = Command::new(env!("CARGO"))
        .args(["run", "-q", "-p", "klint-rs", "--example", "config_keys"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice::<Vec<String>>(&output.stdout)
        .expect("example prints a JSON array of key paths")
        .into_iter()
        .collect()
}

fn read_set(manifest: &Value, field: &str) -> BTreeSet<String> {
    manifest[field]
        .as_array()
        .unwrap_or_else(|| panic!("manifest.{field} is an array"))
        .iter()
        .map(|v| v.as_str().expect("string").to_string())
        .collect()
}

fn manifest() -> Value {
    let text = read_to_string(manifest_path()).expect("tests/config-keys.json exists");
    serde_json::from_str(&text).expect("manifest is JSON")
}

fn refresh_if_requested() {
    if std::env::var("KLINT_KEYS_UPDATE").is_err() {
        return;
    }

    let current = generated();
    let old = read_to_string(manifest_path())
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok());
    let mut retired = BTreeSet::new();

    if let Some(old) = old {
        retired.extend(read_set(&old, "retired"));
        retired.extend(read_set(&old, "current").difference(&current).cloned());
    }

    let body = json!({"current": current, "retired": retired});
    let text = serde_json::to_string_pretty(&body).expect("serialize");
    write(manifest_path(), format!("{text}\n")).expect("write manifest");
}

#[test]
fn generated_keys_equal_the_manifest_current_list() {
    refresh_if_requested();
    let manifest = manifest();
    let current = read_set(&manifest, "current");
    assert_eq!(
        generated(),
        current,
        "tests/config-keys.json is stale — run `KLINT_KEYS_UPDATE=1 cargo test -p klint-rs --test config_keys`"
    );
    let listed: Vec<&str> = manifest["current"]
        .as_array()
        .expect("a")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(
        listed.windows(2).all(|w| w[0] <= w[1]),
        "current must be sorted"
    );

    for key in [
        "$schema",
        "include",
        "arch",
        "rules.recommended",
        "rules.core.no-string-match",
        "rules.core.cognitive-complexity.warn",
        "rules.core.cognitive-complexity.error",
        "rules.core.function-length.warn",
        "rules.core.function-length.error",
        "rules.core.blank-line-after-block.severity",
        "rules.sonar.prefer-at",
    ] {
        assert!(current.contains(key), "current lacks {key}");
    }

    let retired = read_set(&manifest, "retired");

    for key in [
        "rules.core.cognitive-complexity.max",
        "rules.core.cognitive-complexity.severity",
        "rules.core.function-length.max",
        "rules.core.function-length.severity",
        "plugins",
        "rules.sonar/prefer-at",
        "rules.no-string-match",
        "rules.cognitive-complexity",
    ] {
        assert!(retired.contains(key), "retired lacks {key}");
    }

    assert!(current.is_disjoint(&retired));
}

fn retired_flat_rules() -> Vec<String> {
    read_set(&manifest(), "retired")
        .into_iter()
        .filter_map(|k| k.strip_prefix("rules.").map(str::to_string))
        .filter(|name| !name.contains('.'))
        .collect()
}

fn v1_config_planting_every_retired_key() -> String {
    let mut text = String::from("include: [\"src\"]\nplugins: [sonar]\nrules:\n");

    for name in retired_flat_rules() {
        text.push_str(&format!("  {name}: warn\n"));
    }

    text
}

fn hand_written_v2() -> String {
    let mut core = String::new();
    let mut sonar = String::new();

    for name in retired_flat_rules() {
        match name.strip_prefix("sonar/") {
            Some(bare) => sonar.push_str(&format!("    {bare}: warn\n")),
            None => core.push_str(&format!("    {name}: warn\n")),
        }
    }

    format!("include: [\"src\"]\nrules:\n  recommended: true\n  core:\n{core}  sonar:\n{sonar}")
}

#[test]
fn every_retired_key_is_still_accepted_by_v1_behaviour() {
    let retired = read_set(&manifest(), "retired");
    assert!(retired.contains("plugins"));
    let p = project(
        "klint.yaml",
        v1_config_planting_every_retired_key().as_bytes(),
    );
    let ran = klint(&p.root, &["--json"]);
    assert!(ran.status == 0 || ran.status == 2, "{}", ran.all());
    let json: Value = serde_json::from_str(&ran.stdout).expect("json");
    let unknown: Vec<_> = config_warnings(&json)
        .into_iter()
        .filter(|w| {
            w["message"]
                .as_str()
                .is_some_and(|m| m.contains("Unknown config key"))
        })
        .collect();
    assert!(unknown.is_empty(), "{unknown:?}");
}

#[test]
fn migrating_a_config_with_every_retired_key_leaves_none_and_matches_the_hand_written_v2() {
    let p = project(
        "klint.yaml",
        v1_config_planting_every_retired_key().as_bytes(),
    );
    let before = without_config_warnings(&lint_json(&p.root));

    let ran = klint(&p.root, &["migrate"]);

    assert_eq!(ran.status, 0, "{}", ran.all());
    let migrated = read_to_string(p.root.join("klint.yaml")).expect("read");
    assert!(!migrated.contains("plugins"), "{migrated}");
    assert!(!migrated.contains("sonar/"), "{migrated}");
    assert_eq!(migrated, hand_written_v2());
    assert_eq!(without_config_warnings(&lint_json(&p.root)), before);
    assert!(config_warnings(&lint_json(&p.root)).is_empty());
}

#[test]
fn a_second_migrate_changes_nothing() {
    let p = project(
        "klint.yaml",
        v1_config_planting_every_retired_key().as_bytes(),
    );
    klint(&p.root, &["migrate"]);
    let first = std::fs::read(p.root.join("klint.yaml")).expect("read");

    let ran = klint(&p.root, &["migrate"]);

    assert_eq!(ran.status, 0, "{}", ran.all());
    assert_eq!(
        std::fs::read(p.root.join("klint.yaml")).expect("read"),
        first
    );
}

#[test]
fn public_key_paths_list_a_group_entry_then_each_option_path() {
    let paths = klint_rs::config_key_paths();

    for expected in [
        "rules.sonar.prefer-at",
        "rules.sonar.prefer-at.severity",
        "rules.sonar.prefer-at.include",
        "rules.core.no-string-match",
    ] {
        assert!(
            paths.contains(&expected.to_string()),
            "{expected} in {paths:?}"
        );
    }

    let mut sorted = paths.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(paths, sorted, "sorted and deduplicated");
}
