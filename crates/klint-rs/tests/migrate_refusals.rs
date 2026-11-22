mod common;
mod migrate_support;

use migrate_support::*;
use std::fs::read_to_string;

fn yaml(config: &str) -> Project {
    project("klint.yaml", config.as_bytes())
}

fn file(p: &Project, name: &str) -> String {
    read_to_string(p.root.join(name)).expect("read file")
}

fn assert_refused(config: &str, line: usize) {
    let p = yaml(config);
    let ran = klint(&p.root, &["migrate"]);
    assert_ne!(ran.status, 0, "{config}: {}", ran.all());
    assert!(names_line(&ran.all(), line), "{config}: {}", ran.all());
    assert_eq!(file(&p, "klint.yaml"), config);
}

#[test]
fn an_invalid_rule_value_stops_migrate_and_writes_nothing() {
    let config = "include: [\"src\"]\nrules:\n  no-string-match: 42\n";
    let p = yaml(config);
    let ran = klint(&p.root, &["migrate"]);

    assert_ne!(ran.status, 0, "{}", ran.all());
    assert!(ran.all().contains("failed to parse"), "{}", ran.all());
    assert_eq!(file(&p, "klint.yaml"), config);
}

#[test]
fn unknown_plugin_stops_with_an_error_naming_it_and_its_line() {
    let config = "include: [\"src\"]\nplugins: [other]\n";
    let p = yaml(config);
    let ran = klint(&p.root, &["migrate"]);
    assert_ne!(ran.status, 0);
    assert!(ran.all().contains("other"), "{}", ran.all());
    assert!(names_line(&ran.all(), 2), "{}", ran.all());
    assert_eq!(file(&p, "klint.yaml"), config);
}

#[test]
fn anchor_in_rules_is_refused() {
    assert_refused(
        "include: [\"src\"]\nrules:\n  no-string-match: &sev error\n  sonar/prefer-at: *sev\n",
        3,
    );
}

#[test]
fn alias_in_rules_is_refused() {
    assert_refused(
        "include: [\"src\"]\nx: &a error\nrules:\n  no-string-match: *a\n",
        4,
    );
}

#[test]
fn flow_map_rules_on_one_line_is_refused() {
    assert_refused("include: [\"src\"]\nrules: {no-string-match: error}\n", 2);
}

#[test]
fn duplicate_key_in_rules_is_refused() {
    assert_refused(
        "include: [\"src\"]\nrules:\n  no-string-match: error\n  no-string-match: warn\n",
        4,
    );
}

#[test]
fn tag_in_rules_is_refused() {
    assert_refused(
        "include: [\"src\"]\nrules:\n  no-string-match: !!str error\n",
        3,
    );
}

#[test]
fn mixed_flat_rule_and_v2_group_is_refused_listing_the_mixed_keys() {
    let config =
        "include: [\"src\"]\nrules:\n  no-string-match: error\n  core:\n    prefer-at: warn\n";
    let p = yaml(config);
    let ran = klint(&p.root, &["migrate"]);
    assert_ne!(ran.status, 0, "{}", ran.all());
    assert!(ran.all().contains("mixes v1 and v2"), "{}", ran.all());
    assert!(ran.all().contains("no-string-match"), "{}", ran.all());
    assert_eq!(file(&p, "klint.yaml"), config);
}

#[test]
fn plugins_next_to_a_v2_group_is_refused_listing_the_mixed_keys() {
    let config =
        "include: [\"src\"]\nplugins: [sonar]\nrules:\n  core:\n    no-string-match: error\n";
    let p = yaml(config);
    let ran = klint(&p.root, &["migrate"]);
    assert_ne!(ran.status, 0, "{}", ran.all());
    assert!(ran.all().contains("mixes v1 and v2"), "{}", ran.all());
    assert!(ran.all().contains("plugins"), "{}", ran.all());
    assert_eq!(file(&p, "klint.yaml"), config);
}

#[test]
fn refusal_on_a_two_digit_line_names_the_whole_line() {
    let filler = "# c\n".repeat(11);
    let config = format!("{filler}include: [\"src\"]\nplugins: [other]\n");
    assert_refused(&config, 13);
}

#[test]
fn a_refused_migration_during_fix_counts_as_a_warning_in_the_summary() {
    let config =
        "include: [\"src\"]\nplugins: [sonar]\nrules:\n  sonar/prefer-string-raw: &sev off\n";
    let p = yaml(config);
    let ran = klint(&p.root, &["--fix", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&ran.stdout)
        .unwrap_or_else(|err| panic!("json: {err}: {}", ran.all()));
    let warnings = violations(&json)
        .iter()
        .filter(|violation| violation["severity"] == "warn")
        .count();

    assert!(warnings > 0, "{json}");
    assert_eq!(json["summary"]["warnings"], warnings, "{json}");
}
