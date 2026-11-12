mod common;
mod migrate_support;

use migrate_support::*;
use std::fs::{read, read_to_string, write};

const V1: &str = "# yaml-language-server: $schema=./klint.schema.json\n# top comment\n\nplugins: [sonar]\ninclude: [\"src\"]\n# about rules\nrules:\n  # why\n  no-string-match: error   # trailing\n  sonar/prefer-string-raw: off\n  blank-line-before-control-flow: {statements: [if, return]}\n  # block\n  cognitive-complexity:\n    max: 10\n    severity: warn\n  sonar/prefer-at: warn\n\narch: {}\n";

const V2: &str = "# yaml-language-server: $schema=./klint.schema.json\n# top comment\n\ninclude: [\"src\"]\n# about rules\nrules:\n  recommended: true\n  core:\n    # why\n    no-string-match: error   # trailing\n    blank-line-before-control-flow: {statements: [if, return]}\n    # block\n    cognitive-complexity:\n      warn: 10\n  sonar:\n    prefer-string-raw: off\n    prefer-at: warn\n\narch: {}\n";

fn yaml(config: &str) -> Project {
    project("klint.yaml", config.as_bytes())
}

fn file(p: &Project, name: &str) -> String {
    read_to_string(p.root.join(name)).expect("read file")
}

#[test]
fn migrate_rewrites_yaml_keeping_comments_blank_lines_and_values() {
    let p = yaml(V1);
    let before = without_config_warnings(&lint_json(&p.root));

    let ran = klint(&p.root, &["migrate"]);

    assert_eq!(ran.status, 0, "{}", ran.all());
    assert_eq!(file(&p, "klint.yaml"), V2);
    assert_eq!(without_config_warnings(&lint_json(&p.root)), before);
    assert!(config_warnings(&lint_json(&p.root)).is_empty());
}

#[test]
fn second_migrate_reports_already_v2_and_writes_nothing() {
    let p = yaml(V1);
    klint(&p.root, &["migrate"]);
    let after_first = read(p.root.join("klint.yaml")).expect("read");

    let ran = klint(&p.root, &["migrate"]);

    assert_eq!(ran.status, 0, "{}", ran.all());
    assert!(ran.all().contains("already v2"), "{}", ran.all());
    assert_eq!(read(p.root.join("klint.yaml")).expect("read"), after_first);
}

#[test]
fn migrate_check_exits_1_on_v1_and_0_on_v2_and_never_writes() {
    let p = yaml(V1);
    let ran = klint(&p.root, &["migrate", "--check"]);
    assert_eq!(ran.status, 1, "{}", ran.all());
    assert_eq!(file(&p, "klint.yaml"), V1);

    klint(&p.root, &["migrate"]);
    let v2 = read(p.root.join("klint.yaml")).expect("read");
    let ran = klint(&p.root, &["migrate", "--check"]);
    assert_eq!(ran.status, 0, "{}", ran.all());
    assert_eq!(read(p.root.join("klint.yaml")).expect("read"), v2);
}

#[test]
fn migrate_on_a_config_without_rules_or_plugins_is_already_v2() {
    let p = yaml("include: [\"src\"]\n");
    let ran = klint(&p.root, &["migrate"]);
    assert_eq!(ran.status, 0);
    assert!(ran.all().contains("already v2"));
    assert_eq!(file(&p, "klint.yaml"), "include: [\"src\"]\n");
}

#[test]
fn every_plugins_spelling_becomes_recommended_true() {
    for spelling in [
        "plugins: [sonar]\n",
        "plugins: [\"sonar\"]\n",
        "plugins:\n  - sonar\n",
    ] {
        let p = yaml(&format!("include: [\"src\"]\n{spelling}"));
        let ran = klint(&p.root, &["migrate"]);
        assert_eq!(ran.status, 0, "{spelling}: {}", ran.all());
        assert_eq!(
            file(&p, "klint.yaml"),
            "include: [\"src\"]\nrules:\n  recommended: true\n",
            "{spelling}"
        );
    }
}

#[test]
fn empty_plugins_list_is_removed_and_adds_nothing() {
    let p = yaml("include: [\"src\"]\nplugins: []\n");
    let ran = klint(&p.root, &["migrate"]);
    assert_eq!(ran.status, 0, "{}", ran.all());
    assert_eq!(file(&p, "klint.yaml"), "include: [\"src\"]\n");
}

#[test]
fn rule_listed_via_plugin_and_explicitly_keeps_recommended_and_the_explicit_entry() {
    let p = yaml("include: [\"src\"]\nplugins: [sonar]\nrules:\n  sonar/prefer-at: warn\n");
    klint(&p.root, &["migrate"]);
    assert_eq!(
        file(&p, "klint.yaml"),
        "include: [\"src\"]\nrules:\n  recommended: true\n  sonar:\n    prefer-at: warn\n"
    );
}

#[test]
fn comment_above_rules_stays_above_rules() {
    let p = yaml("include: [\"src\"]\n# keep me\nrules:\n  no-string-match: error\n");
    klint(&p.root, &["migrate"]);
    assert_eq!(
        file(&p, "klint.yaml"),
        "include: [\"src\"]\n# keep me\nrules:\n  core:\n    no-string-match: error\n"
    );
}

#[test]
fn group_with_no_rules_is_omitted() {
    let p = yaml("include: [\"src\"]\nrules:\n  sonar/prefer-at: error\n");
    let ran = klint(&p.root, &["migrate"]);
    assert_eq!(ran.status, 0, "{}", ran.all());
    let text = file(&p, "klint.yaml");
    assert!(
        text.contains("sonar:") && !text.contains("core:") && !text.contains("recommended"),
        "{text}"
    );
}

#[test]
fn crlf_and_bom_are_kept() {
    let crlf = V1.replace('\n', "\r\n");
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(crlf.as_bytes());
    let p = project("klint.yaml", &bytes);

    let ran = klint(&p.root, &["migrate"]);

    assert_eq!(ran.status, 0, "{}", ran.all());
    let mut expected = vec![0xEF, 0xBB, 0xBF];
    expected.extend_from_slice(V2.replace('\n', "\r\n").as_bytes());
    assert_eq!(read(p.root.join("klint.yaml")).expect("read"), expected);
}

fn assert_refused(config: &str, line: usize) {
    let p = yaml(config);
    let ran = klint(&p.root, &["migrate"]);
    assert_ne!(ran.status, 0, "{config}: {}", ran.all());
    assert!(names_line(&ran.all(), line), "{config}: {}", ran.all());
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
fn rules_experimental_alone_counts_as_v2_for_migrate_and_check() {
    let config = "include: [\"src\"]\nrules:\n  experimental:\n    x: error\n";
    let p = yaml(config);
    let check = klint(&p.root, &["migrate", "--check"]);
    assert_eq!(check.status, 0, "{}", check.all());
    let ran = klint(&p.root, &["migrate"]);
    assert_eq!(ran.status, 0, "{}", ran.all());
    assert!(ran.all().contains("already v2"), "{}", ran.all());
    assert_eq!(file(&p, "klint.yaml"), config);
}

#[test]
fn fix_with_a_refused_migration_warns_keeps_the_config_and_still_fixes() {
    let config =
        "include: [\"src\"]\nplugins: [sonar]\nrules:\n  sonar/prefer-string-raw: &sev off\n";
    let p = yaml(config);
    let ran = klint(&p.root, &["--fix"]);

    assert_eq!(ran.status, 0, "{}", ran.all());
    assert_eq!(file(&p, "klint.yaml"), config);
    assert!(file(&p, "src/a.ts").contains("arr.at(-1)"));
    assert!(ran.all().contains("klint/config"), "{}", ran.all());
    assert!(names_line(&ran.all(), 4), "{}", ran.all());
}

#[test]
fn migration_refuses_to_write_when_findings_would_differ() {
    let p = yaml(V1);
    let ran = std::process::Command::new(env!("CARGO_BIN_EXE_klint-rs"))
        .args(["migrate", "--config", &p.root.to_string_lossy()])
        .env("KLINT_NO_STATS", "1")
        .env("KLINT_MIGRATE_FORCE_MISMATCH", "1")
        .output()
        .expect("runs");
    assert_eq!(ran.status.code(), Some(1));
    let output = String::from_utf8_lossy(&ran.stderr).to_lowercase();
    assert!(output.contains("finding"), "{output}");
    assert_eq!(file(&p, "klint.yaml"), V1);
}

#[test]
fn fix_migrates_a_v1_config_then_applies_fixes_and_reports_it() {
    let p = yaml("include: [\"src\"]\nplugins: [sonar]\n");
    let ran = klint(&p.root, &["--fix"]);

    assert_eq!(ran.status, 0, "{}", ran.all());
    assert_eq!(
        file(&p, "klint.yaml"),
        "include: [\"src\"]\nrules:\n  recommended: true\n"
    );
    assert!(file(&p, "src/a.ts").contains("arr.at(-1)"));
    assert!(ran.all().to_lowercase().contains("migrat"), "{}", ran.all());
}

#[test]
fn migrates_json_config_with_same_mapping_two_space_indent_original_order() {
    let v1 = "{\n  \"include\": [\"src\"],\n  \"rules\": {\n    \"sonar/prefer-at\": \"warn\",\n    \"no-string-match\": \"error\"\n  },\n  \"plugins\": [\"sonar\"]\n}\n";
    let p = project("klint.config.json", v1.as_bytes());
    let before = without_config_warnings(&lint_json(&p.root));

    let ran = klint(&p.root, &["migrate"]);

    assert_eq!(ran.status, 0, "{}", ran.all());
    let migrated: serde_json::Value =
        serde_json::from_str(&file(&p, "klint.config.json")).expect("json");
    assert_eq!(
        migrated,
        serde_json::json!({
            "include": ["src"],
            "rules": {
                "recommended": true,
                "core": {"no-string-match": "error"},
                "sonar": {"prefer-at": "warn"}
            }
        })
    );
    assert!(file(&p, "klint.config.json").contains("\n  \"include\""));
    assert!(
        file(&p, "klint.config.json").find("include") < file(&p, "klint.config.json").find("rules")
    );
    assert_eq!(without_config_warnings(&lint_json(&p.root)), before);
    let again = klint(&p.root, &["migrate"]);
    assert!(again.all().contains("already v2"));
}

#[test]
fn only_the_config_klint_reads_is_migrated_when_both_exist() {
    let p = yaml("include: [\"src\"]\nplugins: [sonar]\n");
    let json = "{\"include\": [\"src\"], \"plugins\": [\"sonar\"]}\n";
    write(p.root.join("klint.config.json"), json).expect("write");

    klint(&p.root, &["migrate"]);

    assert!(file(&p, "klint.yaml").contains("recommended: true"));
    assert_eq!(file(&p, "klint.config.json"), json);
}

#[test]
fn unknown_rule_name_moves_to_the_core_group() {
    let p = yaml("include: [\"src\"]\nrules:\n  bogus: error\n");
    let ran = klint(&p.root, &["migrate"]);
    assert_eq!(ran.status, 0, "{}", ran.all());
    assert_eq!(
        file(&p, "klint.yaml"),
        "include: [\"src\"]\nrules:\n  core:\n    bogus: error\n"
    );
}

#[test]
fn quoted_glob_with_bang_and_star_in_rules_is_not_refused() {
    let config = "include: [\"src\"]\nrules:\n  no-string-match:\n    severity: error\n    include: [\"!src/**/*.test.ts\"]\n";
    let p = yaml(config);
    let ran = klint(&p.root, &["migrate"]);
    assert_eq!(ran.status, 0, "{}", ran.all());
    assert!(file(&p, "klint.yaml").contains("\"!src/**/*.test.ts\""));
}

#[test]
fn empty_rules_map_is_migrated_without_refusal() {
    let p = yaml("include: [\"src\"]\nrules: {}\n");
    let ran = klint(&p.root, &["migrate"]);
    assert_eq!(ran.status, 0, "{}", ran.all());
}

#[test]
fn double_quoted_rule_key_keeps_its_quotes_when_renamed() {
    let p = yaml("include: [\"src\"]\nrules:\n  \"sonar/prefer-at\": warn\n");
    let ran = klint(&p.root, &["migrate"]);
    assert_eq!(ran.status, 0, "{}", ran.all());
    assert!(
        file(&p, "klint.yaml").contains("    \"prefer-at\": warn\n"),
        "{}",
        file(&p, "klint.yaml")
    );
}

#[test]
fn refusal_on_a_two_digit_line_names_the_whole_line() {
    let filler = "# c\n".repeat(11);
    let config = format!("{filler}include: [\"src\"]\nplugins: [other]\n");
    assert_refused(&config, 13);
}

#[test]
fn json_plugins_prop_becomes_rules_in_its_own_position_when_rules_is_absent() {
    let v1 = "{\n  \"plugins\": [\"sonar\"],\n  \"include\": [\"src\"]\n}\n";
    let p = project("klint.config.json", v1.as_bytes());
    let ran = klint(&p.root, &["migrate"]);
    assert_eq!(ran.status, 0, "{}", ran.all());
    let text = file(&p, "klint.config.json");
    let migrated: serde_json::Value = serde_json::from_str(&text).expect("json");
    assert_eq!(
        migrated,
        serde_json::json!({"rules": {"recommended": true}, "include": ["src"]})
    );
    assert!(text.find("rules") < text.find("include"), "{text}");
}

#[test]
fn json_empty_plugins_list_is_dropped_and_adds_nothing() {
    let v1 = "{\n  \"include\": [\"src\"],\n  \"plugins\": []\n}\n";
    let p = project("klint.config.json", v1.as_bytes());
    let ran = klint(&p.root, &["migrate"]);
    assert_eq!(ran.status, 0, "{}", ran.all());
    let migrated: serde_json::Value =
        serde_json::from_str(&file(&p, "klint.config.json")).expect("json");
    assert_eq!(migrated, serde_json::json!({"include": ["src"]}));
}

#[test]
fn json_unknown_plugin_is_refused_naming_its_line() {
    let v1 = "{\n  \"include\": [\"src\"],\n  \"plugins\": [\"other\"]\n}\n";
    let p = project("klint.config.json", v1.as_bytes());
    let ran = klint(&p.root, &["migrate"]);
    assert_ne!(ran.status, 0, "{}", ran.all());
    assert!(names_line(&ran.all(), 3), "{}", ran.all());
    assert_eq!(file(&p, "klint.config.json"), v1);
}

#[test]
fn json_with_a_non_object_root_is_refused_and_unchanged() {
    let v1 = "[1, 2]\n";
    let p = project("klint.config.json", v1.as_bytes());
    let ran = klint(&p.root, &["migrate"]);
    assert_ne!(ran.status, 0, "{}", ran.all());
    assert_eq!(file(&p, "klint.config.json"), v1);
}
