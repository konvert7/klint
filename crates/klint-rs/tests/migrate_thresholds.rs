mod common;
mod migrate_support;

use migrate_support::*;
use std::fs::{create_dir_all, read, read_to_string, write};

const PRE: &str = "include: [\"src\"]\n# top\nrules:\n  core:\n    # above\n";
const POST: &str = "    no-string-match: error # after\n";

fn cc_function(score: usize) -> String {
    format!(
        "function big(a) {{\n{}}}\n",
        "  if (a) {\n    g();\n  }\n".repeat(score)
    )
}

fn fl_function(lines: usize) -> String {
    format!("function long() {{\n{}}}\n", "  a();\n".repeat(lines))
}

fn source() -> String {
    format!(
        "const hit = \"abc\".match(/a/);\n{}{}",
        cc_function(16),
        fl_function(90)
    )
}

fn with_findings(file: &str, config: &str) -> Project {
    let p = project(file, config.as_bytes());
    create_dir_all(p.root.join("src")).expect("dir");
    write(p.root.join("src/a.ts"), source()).expect("write source");
    p
}

fn text(p: &Project, name: &str) -> String {
    read_to_string(p.root.join(name)).expect("read")
}

fn assert_migrates(old: &str, new: &str) {
    let p = with_findings("klint.yaml", &format!("{PRE}{old}{POST}"));
    let before = lint_json(&p.root);
    assert!(!without_config_warnings(&before).is_empty(), "{old}");

    let ran = klint(&p.root, &["migrate"]);

    assert_eq!(ran.status, 0, "{old}: {}", ran.all());
    assert_eq!(text(&p, "klint.yaml"), format!("{PRE}{new}{POST}"), "{old}");
    let after = lint_json(&p.root);
    assert_eq!(
        serde_json::to_string(&without_config_warnings(&after)).expect("s"),
        serde_json::to_string(&without_config_warnings(&before)).expect("s"),
        "{old}"
    );
    assert!(config_warnings(&after).is_empty(), "{old}: {after}");
    let written = read(p.root.join("klint.yaml")).expect("read");
    let again = klint(&p.root, &["migrate"]);
    assert_eq!(again.status, 0, "{old}: {}", again.all());
    assert_eq!(read(p.root.join("klint.yaml")).expect("read"), written);
}

#[test]
fn block_style_rows_migrate_for_cognitive_complexity() {
    let rows = [
        (
            "    cognitive-complexity:\n      max: 10\n",
            "    cognitive-complexity:\n      error: 10\n",
        ),
        (
            "    cognitive-complexity:\n      max: 10\n      severity: error\n",
            "    cognitive-complexity:\n      error: 10\n",
        ),
        (
            "    cognitive-complexity:\n      max: 10   # trailing\n      severity: warn\n",
            "    cognitive-complexity:\n      warn: 10   # trailing\n",
        ),
        (
            "    cognitive-complexity:\n      severity: warn\n      max: 10\n",
            "    cognitive-complexity:\n      warn: 10\n",
        ),
        (
            "    cognitive-complexity:\n      max: 10\n      severity: off\n",
            "    cognitive-complexity: off\n",
        ),
        (
            "    cognitive-complexity:\n      severity: warn\n",
            "    cognitive-complexity:\n      warn: 15\n",
        ),
        (
            "    cognitive-complexity:\n      severity: error\n",
            "    cognitive-complexity:\n      error: 15\n",
        ),
        (
            "    cognitive-complexity:\n      severity: off\n",
            "    cognitive-complexity: off\n",
        ),
        (
            "    cognitive-complexity:\n      max: 10\n      severity: warn\n      include: [\"src/**\"]\n",
            "    cognitive-complexity:\n      warn: 10\n      include: [\"src/**\"]\n",
        ),
    ];

    for (old, new) in rows {
        assert_migrates(old, new);
    }
}

#[test]
fn block_style_rows_migrate_for_function_length() {
    let rows = [
        (
            "    function-length:\n      max: 40\n",
            "    function-length:\n      error: 40\n",
        ),
        (
            "    function-length:\n      max: 40\n      severity: warn\n",
            "    function-length:\n      warn: 40\n",
        ),
        (
            "    function-length:\n      max: 40\n      severity: off\n",
            "    function-length: off\n",
        ),
        (
            "    function-length:\n      severity: warn\n",
            "    function-length:\n      warn: 50\n",
        ),
        (
            "    function-length:\n      severity: error\n",
            "    function-length:\n      error: 50\n",
        ),
        (
            "    function-length:\n      max: 40\n      severity: warn\n      skipBlankLines: true\n      include: [\"src/**\"]\n",
            "    function-length:\n      warn: 40\n      skipBlankLines: true\n      include: [\"src/**\"]\n",
        ),
    ];

    for (old, new) in rows {
        assert_migrates(old, new);
    }
}

#[test]
fn flow_style_rule_values_migrate_as_flow_maps() {
    let rows = [
        (
            "    cognitive-complexity: {max: 10}\n",
            "    cognitive-complexity: {error: 10}\n",
        ),
        (
            "    cognitive-complexity: {max: 10, severity: error}\n",
            "    cognitive-complexity: {error: 10}\n",
        ),
        (
            "    cognitive-complexity: {max: 10, severity: warn}\n",
            "    cognitive-complexity: {warn: 10}\n",
        ),
        (
            "    cognitive-complexity: {max: 10, severity: off}\n",
            "    cognitive-complexity: off\n",
        ),
        (
            "    cognitive-complexity: {severity: warn}\n",
            "    cognitive-complexity: {warn: 15}\n",
        ),
        (
            "    function-length: {severity: error}\n",
            "    function-length: {error: 50}\n",
        ),
        (
            "    function-length: {max: 40, severity: warn, skipBlankLines: true}   # t\n",
            "    function-length: {warn: 40, skipBlankLines: true}   # t\n",
        ),
        (
            "    cognitive-complexity: {max: 5, severity: warn, include: [\"src/**\"]}\n",
            "    cognitive-complexity: {warn: 5, include: [\"src/**\"]}\n",
        ),
    ];

    for (old, new) in rows {
        assert_migrates(old, new);
    }
}

#[test]
fn comment_on_the_rule_line_is_kept() {
    assert_migrates(
        "    cognitive-complexity:   # why\n      max: 10\n",
        "    cognitive-complexity:   # why\n      error: 10\n",
    );
}

#[test]
fn json_rows_migrate_with_identical_findings_and_a_quiet_second_run() {
    let rows = [
        (r#"{"max": 10}"#, r#"{"error": 10}"#),
        (r#"{"max": 10, "severity": "error"}"#, r#"{"error": 10}"#),
        (r#"{"max": 10, "severity": "warn"}"#, r#"{"warn": 10}"#),
        (r#"{"max": 10, "severity": "off"}"#, r#""off""#),
        (r#"{"severity": "warn"}"#, r#"{"warn": 15}"#),
        (r#"{"severity": "off"}"#, r#""off""#),
        (
            r#"{"max": 10, "severity": "warn", "include": ["src/**"]}"#,
            r#"{"warn": 10, "include": ["src/**"]}"#,
        ),
    ];

    for (old, new) in rows {
        let config = format!(
            "{{\n  \"include\": [\"src\"],\n  \"rules\": {{\n    \"core\": {{\n      \"cognitive-complexity\": {old}\n    }}\n  }}\n}}\n"
        );
        let p = with_findings("klint.config.json", &config);
        let before = without_config_warnings(&lint_json(&p.root));

        let ran = klint(&p.root, &["migrate"]);

        assert_eq!(ran.status, 0, "{old}: {}", ran.all());
        let migrated: serde_json::Value =
            serde_json::from_str(&text(&p, "klint.config.json")).expect("json");
        let want: serde_json::Value = serde_json::from_str(new).expect("json");
        assert_eq!(
            migrated["rules"]["core"]["cognitive-complexity"], want,
            "{old}"
        );
        assert_eq!(
            without_config_warnings(&lint_json(&p.root)),
            before,
            "{old}"
        );
        let written = read(p.root.join("klint.config.json")).expect("read");
        klint(&p.root, &["migrate"]);
        assert_eq!(
            read(p.root.join("klint.config.json")).expect("read"),
            written
        );
    }
}

#[test]
fn json_function_length_keeps_skip_blank_lines_and_include() {
    let config = "{\n  \"include\": [\"src\"],\n  \"rules\": {\n    \"core\": {\n      \"function-length\": {\"max\": 40, \"severity\": \"warn\", \"skipBlankLines\": true}\n    }\n  }\n}\n";
    let p = with_findings("klint.config.json", config);
    let before = without_config_warnings(&lint_json(&p.root));

    let ran = klint(&p.root, &["migrate"]);

    assert_eq!(ran.status, 0, "{}", ran.all());
    let migrated: serde_json::Value =
        serde_json::from_str(&text(&p, "klint.config.json")).expect("json");
    assert_eq!(
        migrated["rules"]["core"]["function-length"],
        serde_json::json!({"warn": 40, "skipBlankLines": true})
    );
    assert_eq!(without_config_warnings(&lint_json(&p.root)), before);
}

#[test]
fn a_v1_config_with_sonar_rules_and_max_migrates_both_steps_in_one_run() {
    let v1 = "include: [\"src\"]\nplugins: [sonar]\nrules:\n  sonar/prefer-at: warn\n  cognitive-complexity:\n    max: 10\n    severity: warn\n";
    let v2 = "include: [\"src\"]\nrules:\n  recommended: true\n  core:\n    cognitive-complexity:\n      warn: 10\n  sonar:\n    prefer-at: warn\n";
    let p = with_findings("klint.yaml", v1);
    let before = without_config_warnings(&lint_json(&p.root));

    let ran = klint(&p.root, &["migrate"]);

    assert_eq!(ran.status, 0, "{}", ran.all());
    assert_eq!(text(&p, "klint.yaml"), v2);
    assert_eq!(without_config_warnings(&lint_json(&p.root)), before);
    assert!(config_warnings(&lint_json(&p.root)).is_empty());
    klint(&p.root, &["migrate"]);
    assert_eq!(text(&p, "klint.yaml"), v2);
}

const V2_WITH_MAX: &str =
    "include: [\"src\"]\nrules:\n  core:\n    cognitive-complexity:\n      max: 10\n";

#[test]
fn migrate_check_exits_1_for_a_v2_config_with_max_and_0_after_migration() {
    let p = with_findings("klint.yaml", V2_WITH_MAX);

    let check = klint(&p.root, &["migrate", "--check"]);
    assert_eq!(check.status, 1, "{}", check.all());
    assert!(
        check.all().contains("uses an old config form"),
        "{}",
        check.all()
    );
    assert!(!check.all().contains("v1 format"), "{}", check.all());
    assert_eq!(text(&p, "klint.yaml"), V2_WITH_MAX);

    let ran = klint(&p.root, &["migrate"]);
    assert_eq!(ran.status, 0, "{}", ran.all());
    let check = klint(&p.root, &["migrate", "--check"]);
    assert_eq!(check.status, 0, "{}", check.all());
}

#[test]
fn migrate_check_stays_0_for_a_v2_config_already_in_the_new_form() {
    let config = "include: [\"src\"]\nrules:\n  core:\n    cognitive-complexity:\n      - {error: 15}\n      - {warn: 10, include: [\"src/**\"]}\n    function-length: {warn: 60, error: 80}\n";
    let p = with_findings("klint.yaml", config);
    let check = klint(&p.root, &["migrate", "--check"]);
    assert_eq!(check.status, 0, "{}", check.all());
    let ran = klint(&p.root, &["migrate"]);
    assert_eq!(ran.status, 0, "{}", ran.all());
    assert_eq!(text(&p, "klint.yaml"), config);
}

#[test]
fn fix_also_runs_the_threshold_migration() {
    let p = with_findings("klint.yaml", V2_WITH_MAX);

    let ran = klint(&p.root, &["--fix"]);

    assert_eq!(ran.status, 0, "{}", ran.all());
    assert_eq!(
        text(&p, "klint.yaml"),
        "include: [\"src\"]\nrules:\n  core:\n    cognitive-complexity:\n      error: 10\n"
    );
}

#[test]
fn migration_refuses_to_write_when_findings_would_differ() {
    let p = with_findings("klint.yaml", V2_WITH_MAX);
    let ran = std::process::Command::new(env!("CARGO_BIN_EXE_klint-rs"))
        .args(["migrate", "--config", &p.root.to_string_lossy()])
        .env("KLINT_NO_STATS", "1")
        .env("KLINT_MIGRATE_FORCE_MISMATCH", "1")
        .output()
        .expect("runs");
    assert_eq!(ran.status.code(), Some(1));
    assert_eq!(text(&p, "klint.yaml"), V2_WITH_MAX);
}

fn assert_refused(config: &str, line: usize) {
    let p = with_findings("klint.yaml", config);
    let ran = klint(&p.root, &["migrate"]);
    assert_ne!(ran.status, 0, "{config}: {}", ran.all());
    assert!(names_line(&ran.all(), line), "{config}: {}", ran.all());
    assert_eq!(text(&p, "klint.yaml"), config);
}

#[test]
fn anchor_on_a_threshold_rule_is_refused_and_the_file_is_unchanged() {
    assert_refused(
        "include: [\"src\"]\nrules:\n  core:\n    cognitive-complexity: &lim\n      max: 10\n",
        4,
    );
}

#[test]
fn alias_on_a_threshold_rule_is_refused_and_the_file_is_unchanged() {
    assert_refused(
        "include: [\"src\"]\nx: &a {max: 10}\nrules:\n  core:\n    function-length: *a\n",
        5,
    );
}

#[test]
fn duplicate_max_key_is_refused_and_the_file_is_unchanged() {
    assert_refused(
        "include: [\"src\"]\nrules:\n  core:\n    cognitive-complexity:\n      max: 10\n      max: 12\n",
        6,
    );
}
