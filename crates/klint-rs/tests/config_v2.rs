mod common;
mod migrate_support;

use migrate_support::*;
use std::fs::{read, write};

const ALL_SIX: &str = "const arr: number[] = [];\nlet x: object | undefined;\nconst a = arr[arr.length - 1];\nif (x == null) x = {};\nconst p = \"C:\\\\Users\\\\file.txt\";\nconst r = new RegExp(`\\\\.foo`);\ndeclare const text: string;\nconst t = text.replace(/foo/g, \"b\");\nconst c = /a[b]c/;\n";

fn with_source(config: &str, source: &str) -> Project {
    let p = project("klint.yaml", config.as_bytes());
    write(p.root.join("src/a.ts"), source).expect("write source");
    p
}

const V1_PLUGIN: &str = "include: [\"src\"]\nplugins: [sonar]\n";
const V2_RECOMMENDED: &str = "include: [\"src\"]\nrules:\n  recommended: true\n";

fn rule_ids(json: &serde_json::Value) -> Vec<String> {
    let mut ids: Vec<String> = without_config_warnings(json)
        .iter()
        .map(|v| v["rule"].as_str().expect("rule").to_string())
        .collect();
    ids.sort();
    ids
}

#[test]
fn recommended_true_matches_plugins_sonar_on_all_six_rules() {
    let v1 = lint_json(&with_source(V1_PLUGIN, ALL_SIX).root);
    let v2 = lint_json(&with_source(V2_RECOMMENDED, ALL_SIX).root);

    assert_eq!(without_config_warnings(&v1), without_config_warnings(&v2));
    let ids = rule_ids(&v2);

    for rule in [
        "sonar/prefer-string-replaceall",
        "sonar/prefer-string-raw-regexp",
        "sonar/prefer-string-raw",
        "sonar/prefer-nullish-coalescing-assign",
        "sonar/no-single-char-class",
        "sonar/prefer-at",
    ] {
        assert!(ids.iter().any(|i| i == rule), "{rule} missing in {ids:?}");
    }

    assert!(
        without_config_warnings(&v2)
            .iter()
            .all(|v| v["severity"] == "error")
    );
}

#[test]
fn recommended_false_enables_nothing() {
    let p = with_source(
        "include: [\"src\"]\nrules:\n  recommended: false\n",
        ALL_SIX,
    );
    assert!(without_config_warnings(&lint_json(&p.root)).is_empty());
}

#[test]
fn a_group_entry_overrides_recommended() {
    let config =
        "include: [\"src\"]\nrules:\n  recommended: true\n  sonar:\n    prefer-string-raw: off\n";
    let ids = rule_ids(&lint_json(&with_source(config, ALL_SIX).root));
    assert!(
        !ids.contains(&"sonar/prefer-string-raw".to_string()),
        "{ids:?}"
    );
    assert!(ids.contains(&"sonar/prefer-at".to_string()), "{ids:?}");
}

#[test]
fn v2_core_and_sonar_groups_keep_output_rule_ids() {
    let config = "include: [\"src\"]\nrules:\n  core:\n    no-string-match: error\n  sonar:\n    prefer-at: warn\n";
    let json = lint_json(&project_with(config).root);
    let found: Vec<(String, String)> = without_config_warnings(&json)
        .iter()
        .map(|v| {
            (
                v["rule"].as_str().expect("r").into(),
                v["severity"].as_str().expect("s").into(),
            )
        })
        .collect();
    assert!(
        found.contains(&("no-string-match".into(), "error".into())),
        "{found:?}"
    );
    assert!(
        found.contains(&("sonar/prefer-at".into(), "warn".into())),
        "{found:?}"
    );
    assert!(config_warnings(&json).is_empty(), "{json}");
}

fn project_with(config: &str) -> Project {
    project("klint.yaml", config.as_bytes())
}

#[test]
fn v1_config_lints_identically_warns_once_and_is_not_written() {
    let config = "include: [\"src\"]\nplugins: [sonar]\nrules:\n  no-string-match: error\n";
    let p = project_with(config);
    let before = read(p.root.join("klint.yaml")).expect("read");

    let json = lint_json(&p.root);
    let again = klint(&p.root, &[]);

    let warnings = config_warnings(&json);
    assert_eq!(warnings.len(), 1, "{json}");
    assert_eq!(warnings[0]["line"], 1);
    assert_eq!(warnings[0]["severity"], "warn");
    assert_eq!(
        warnings[0]["message"],
        "This config uses the v1 format — run `klint migrate` to upgrade it in place."
    );
    assert!(again.all().contains("klint migrate"), "{}", again.all());
    assert_eq!(read(p.root.join("klint.yaml")).expect("read"), before);
    let ids = rule_ids(&json);
    assert!(
        ids.contains(&"no-string-match".to_string())
            && ids.contains(&"sonar/prefer-at".to_string())
    );
}

#[test]
fn v2_config_gets_no_v1_warning() {
    let json = lint_json(&project_with(V2_RECOMMENDED).root);
    assert!(config_warnings(&json).is_empty(), "{json}");
}

#[test]
fn config_without_rules_and_plugins_is_v2() {
    let json = lint_json(&project_with("include: [\"src\"]\n").root);
    assert!(config_warnings(&json).is_empty(), "{json}");
}

#[test]
fn mixed_flat_rule_and_v2_group_warns_once_and_v2_wins() {
    let config =
        "include: [\"src\"]\nrules:\n  no-string-match: error\n  sonar:\n    prefer-at: error\n";
    let json = lint_json(&project_with(config).root);
    let warnings = config_warnings(&json);
    assert_eq!(warnings.len(), 1, "{json}");
    let message = warnings[0]["message"].as_str().expect("message");
    assert!(
        message.contains("no-string-match") && message.contains("sonar"),
        "{message}"
    );
    assert_eq!(rule_ids(&json), vec!["sonar/prefer-at"]);
}

#[test]
fn plugins_next_to_a_v2_key_warns_once_and_plugins_are_ignored() {
    let config =
        "include: [\"src\"]\nplugins: [sonar]\nrules:\n  core:\n    no-string-match: error\n";
    let json = lint_json(&project_with(config).root);
    let warnings = config_warnings(&json);
    assert_eq!(warnings.len(), 1, "{json}");
    assert!(
        warnings[0]["message"]
            .as_str()
            .expect("m")
            .contains("plugins")
    );
    assert_eq!(rule_ids(&json), vec!["no-string-match"]);
}

#[test]
fn unknown_key_under_rules_warns_naming_it() {
    let config = "include: [\"src\"]\nrules:\n  core:\n    no-string-match: error\n  bogus: 1\n";
    let warnings = config_warnings(&lint_json(&project_with(config).root));
    assert!(
        warnings
            .iter()
            .any(|w| w["message"].as_str().expect("m").contains("bogus")),
        "{warnings:?}"
    );
}

#[test]
fn v1_name_inside_a_v2_group_says_where_it_moved() {
    let config = "include: [\"src\"]\nrules:\n  sonar:\n    sonar/prefer-at: error\n";
    let json = lint_json(&project_with(config).root);
    let messages: Vec<String> = violations(&json)
        .iter()
        .filter(|v| v["rule"] == "klint/unsupported-rule")
        .map(|v| v["message"].as_str().expect("m").to_string())
        .collect();
    assert!(
        messages.iter().any(|m| m
            .contains("\"sonar/prefer-at\" is now rules.sonar.prefer-at — run `klint migrate`.")),
        "{messages:?}"
    );
}

#[test]
fn core_rule_name_inside_the_sonar_group_says_where_it_moved() {
    let config = "include: [\"src\"]\nrules:\n  sonar:\n    no-string-match: error\n";
    let json = lint_json(&project_with(config).root);
    let hit = violations(&json).iter().any(|v| {
        v["message"]
            .as_str()
            .is_some_and(|m| m.contains("rules.core.no-string-match"))
    });
    assert!(hit, "{json}");
}

#[test]
fn sonar_rule_with_prefix_in_core_group_says_where_it_moved() {
    let config = "include: [\"src\"]\nrules:\n  core:\n    sonar/prefer-at: error\n";
    let json = lint_json(&project_with(config).root);
    let hit = violations(&json).iter().any(|v| {
        v["message"]
            .as_str()
            .is_some_and(|m| m.contains("rules.sonar.prefer-at"))
    });
    assert!(hit, "{json}");
}

fn v1_warning_count(config: &str) -> usize {
    let json = lint_json(&project_with(config).root);
    config_warnings(&json)
        .iter()
        .filter(|w| w["message"].as_str().expect("m").contains("v1 format"))
        .count()
}

#[test]
fn plugins_alone_is_v1() {
    assert_eq!(
        v1_warning_count("include: [\"src\"]\nplugins: [sonar]\n"),
        1
    );
}

#[test]
fn flat_known_rule_is_v1() {
    assert_eq!(
        v1_warning_count("include: [\"src\"]\nrules:\n  no-string-match: error\n"),
        1
    );
}

#[test]
fn flat_unknown_name_is_v1_as_today() {
    assert_eq!(
        v1_warning_count("include: [\"src\"]\nrules:\n  bogus: error\n"),
        1
    );
}

#[test]
fn recommended_alone_is_v2() {
    assert_eq!(
        v1_warning_count("include: [\"src\"]\nrules:\n  recommended: true\n"),
        0
    );
}

#[test]
fn empty_rules_map_is_v2() {
    assert_eq!(v1_warning_count("include: [\"src\"]\nrules: {}\n"), 0);
}

#[test]
fn experimental_with_a_flat_unknown_name_is_v2_and_warns_for_the_unknown_key() {
    let config = "include: [\"src\"]\nrules:\n  experimental:\n    x: error\n  bogus: error\n";
    assert_eq!(v1_warning_count(config), 0);
    let warnings = config_warnings(&lint_json(&project_with(config).root));
    assert!(
        warnings
            .iter()
            .any(|w| w["message"].as_str().expect("m").contains("bogus")),
        "{warnings:?}"
    );
}

#[test]
fn plugins_next_to_core_group_and_unknown_key_warn_for_both() {
    let config = "include: [\"src\"]\nplugins: [sonar]\nrules:\n  core:\n    no-string-match: error\n  bogus: 1\n";
    let json = lint_json(&project_with(config).root);
    let warnings = config_warnings(&json);
    assert_eq!(warnings.len(), 2, "{warnings:?}");
    assert!(
        warnings[0]["message"]
            .as_str()
            .expect("m")
            .contains("plugins")
    );
    assert!(
        warnings[1]["message"]
            .as_str()
            .expect("m")
            .contains("bogus")
    );
    assert_eq!(rule_ids(&json), vec!["no-string-match"]);
}

#[test]
fn unknown_key_warning_is_on_its_own_line() {
    let config = "include: [\"src\"]\nrules:\n  core:\n    no-string-match: error\n  bogus: 1\n";
    let warnings = config_warnings(&lint_json(&project_with(config).root));
    let bogus = warnings
        .iter()
        .find(|w| w["message"].as_str().expect("m").contains("bogus"))
        .expect("bogus warning");
    assert_eq!(bogus["line"], 5);
}

#[test]
fn recommended_true_with_one_sonar_rule_off_enables_the_other_five() {
    let config = "include: [\"src\"]\nrules:\n  recommended: true\n  sonar:\n    prefer-at: off\n";
    let json = lint_json(&with_source(config, ALL_SIX).root);
    let ids = rule_ids(&json);
    assert!(!ids.contains(&"sonar/prefer-at".to_string()), "{ids:?}");
    assert!(
        ids.contains(&"sonar/no-single-char-class".to_string()),
        "{ids:?}"
    );
}

#[test]
fn non_boolean_recommended_is_rejected_by_the_binary() {
    let p = project_with("include: [\"src\"]\nrules:\n  recommended: yes-please\n");
    let ran = klint(&p.root, &[]);
    assert_ne!(ran.status, 0, "{}", ran.all());
}

#[test]
fn non_object_core_value_is_rejected_by_the_binary() {
    let p = project_with("include: [\"src\"]\nrules:\n  core: 3\n");
    let ran = klint(&p.root, &[]);
    assert_ne!(ran.status, 0, "{}", ran.all());
}

#[test]
fn unknown_plugin_is_rejected_by_name() {
    let p = project_with("include: [\"src\"]\nplugins: [nope]\n");
    let ran = klint(&p.root, &[]);
    assert_ne!(ran.status, 0, "{}", ran.all());
    assert!(ran.all().contains("nope"), "{}", ran.all());
}

#[test]
fn byte_order_mark_does_not_hide_include() {
    let p = project(
        "klint.yaml",
        b"\xEF\xBB\xBFinclude: [\"src\"]\nrules:\n  recommended: true\n",
    );
    write(p.root.join("src/a.ts"), ALL_SIX).expect("write source");
    let json = lint_json(&p.root);
    assert!(config_warnings(&json).is_empty(), "{json}");
    assert!(
        rule_ids(&json).contains(&"sonar/prefer-at".to_string()),
        "{json}"
    );
}
