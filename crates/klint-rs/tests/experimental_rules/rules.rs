use crate::migrate_support::*;
use crate::support::*;
use std::fs::{read, read_to_string};

#[test]
fn listing_a_rule_under_experimental_makes_it_report() {
    let config = experimental("    no-date-equality: error\n    no-floating-promise: warn\n");
    let json = lint_json(&with_source(&config, BOTH).root);
    assert_eq!(findings_of(&json, DATE_RULE).len(), 1, "{json}");
    assert_eq!(findings_of(&json, PROMISE_RULE).len(), 1, "{json}");
    assert_eq!(findings_of(&json, DATE_RULE)[0]["severity"], "error");
    assert_eq!(findings_of(&json, PROMISE_RULE)[0]["severity"], "warn");
    assert!(rule_warnings(&json).is_empty(), "{json}");
}

#[test]
fn recommended_true_never_enables_an_experimental_rule() {
    let config = "include: [\"src\"]\nrules:\n  recommended: true\n";
    let json = lint_json(&with_source(config, BOTH).root);
    assert!(findings_of(&json, DATE_RULE).is_empty(), "{json}");
    assert!(findings_of(&json, PROMISE_RULE).is_empty(), "{json}");
}

#[test]
fn core_entries_alone_never_enable_an_experimental_rule() {
    let config = "include: [\"src\"]\nrules:\n  core:\n    no-string-match: error\n";
    let json = lint_json(&with_source(config, BOTH).root);
    assert!(findings_of(&json, DATE_RULE).is_empty(), "{json}");
    assert!(findings_of(&json, PROMISE_RULE).is_empty(), "{json}");
}

#[test]
fn recommended_with_one_experimental_entry_enables_only_that_rule() {
    let config = "include: [\"src\"]\nrules:\n  recommended: true\n  experimental:\n    no-date-equality: error\n";
    let json = lint_json(&with_source(config, BOTH).root);
    assert_eq!(findings_of(&json, DATE_RULE).len(), 1, "{json}");
    assert!(findings_of(&json, PROMISE_RULE).is_empty(), "{json}");
}

#[test]
fn experimental_off_enables_nothing() {
    let config = experimental("    no-date-equality: off\n    no-floating-promise: off\n");
    let json = lint_json(&with_source(&config, BOTH).root);
    assert!(findings_of(&json, DATE_RULE).is_empty(), "{json}");
    assert!(findings_of(&json, PROMISE_RULE).is_empty(), "{json}");
    assert!(rule_warnings(&json).is_empty(), "{json}");
}

#[test]
fn rule_ids_carry_no_experimental_prefix() {
    let config = experimental("    no-date-equality: error\n    no-floating-promise: error\n");
    let json = lint_json(&with_source(&config, BOTH).root);
    let ids: Vec<String> = without_config_warnings(&json)
        .iter()
        .map(|v| v["rule"].as_str().expect("rule").to_string())
        .collect();
    assert!(ids.contains(&DATE_RULE.to_string()), "{ids:?}");
    assert!(ids.contains(&PROMISE_RULE.to_string()), "{ids:?}");
    assert!(ids.iter().all(|i| !i.contains("experimental")), "{ids:?}");
}

#[test]
fn text_output_uses_the_plain_rule_id() {
    let config = experimental("    no-date-equality: error\n");
    let ran = klint(&with_source(&config, DATES).root, &[]);
    assert!(ran.all().contains(DATE_RULE), "{}", ran.all());
    assert!(!ran.all().contains("experimental/"), "{}", ran.all());
}

#[test]
fn stats_file_uses_the_plain_rule_id_and_stays_prefix_free_on_a_second_run() {
    let config = experimental("    no-date-equality: error\n");
    let p = with_source(&config, DATES);

    for _ in 0..2 {
        std::process::Command::new(env!("CARGO_BIN_EXE_klint-rs"))
            .args(["--config", &p.root.to_string_lossy()])
            .env_remove("KLINT_NO_STATS")
            .output()
            .expect("klint-rs runs");
    }

    let stats = read_to_string(p.root.join(".klint/stats.json")).expect("stats written");
    assert!(stats.contains(DATE_RULE), "{stats}");
    assert!(!stats.contains("experimental/"), "{stats}");
}

#[test]
fn the_stats_command_lists_the_plain_rule_id() {
    let config = experimental("    no-date-equality: error\n");
    let p = with_source(&config, DATES);
    std::process::Command::new(env!("CARGO_BIN_EXE_klint-rs"))
        .args(["--config", &p.root.to_string_lossy()])
        .env_remove("KLINT_NO_STATS")
        .output()
        .expect("klint-rs runs");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_klint-rs"))
        .args(["stats", "--config", &p.root.to_string_lossy()])
        .output()
        .expect("klint-rs stats runs");
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(text.contains(DATE_RULE), "{text}");
    assert!(!text.contains("experimental/"), "{text}");
}

#[test]
fn experimental_rule_under_core_warns_once_naming_rule_and_group_and_is_ignored() {
    let config = "include: [\"src\"]\nrules:\n  core:\n    no-date-equality: error\n";
    let json = lint_json(&with_source(config, DATES).root);
    let warnings = warnings_naming(&json, DATE_RULE);
    assert_eq!(warnings.len(), 1, "{json}");
    assert!(warnings[0].contains("experimental"), "{warnings:?}");
    assert!(findings_of(&json, DATE_RULE).is_empty(), "{json}");
}

#[test]
fn experimental_rule_under_sonar_warns_once_naming_rule_and_group_and_is_ignored() {
    let config = "include: [\"src\"]\nrules:\n  sonar:\n    no-floating-promise: error\n";
    let json = lint_json(&with_source(config, DROPPED).root);
    let warnings = warnings_naming(&json, PROMISE_RULE);
    assert_eq!(warnings.len(), 1, "{json}");
    assert!(warnings[0].contains("experimental"), "{warnings:?}");
    assert!(findings_of(&json, PROMISE_RULE).is_empty(), "{json}");
}

#[test]
fn core_rule_under_experimental_warns_once_naming_rule_and_group_and_is_ignored() {
    let config = experimental("    no-string-match: error\n");
    let json = lint_json(&project("klint.yaml", config.as_bytes()).root);
    let warnings = warnings_naming(&json, "no-string-match");
    assert_eq!(warnings.len(), 1, "{json}");
    assert!(warnings[0].contains("core"), "{warnings:?}");
    assert!(findings_of(&json, "no-string-match").is_empty(), "{json}");
}

#[test]
fn sonar_rule_under_experimental_warns_once_naming_rule_and_group_and_is_ignored() {
    let config = experimental("    prefer-at: error\n");
    let source = "const arr: number[] = [];\nconst a = arr[arr.length - 1];\n";
    let json = lint_json(&with_source(&config, source).root);
    let warnings = warnings_naming(&json, "prefer-at");
    assert_eq!(warnings.len(), 1, "{json}");
    assert!(warnings[0].contains("sonar"), "{warnings:?}");
    assert!(findings_of(&json, "sonar/prefer-at").is_empty(), "{json}");
}

#[test]
fn unknown_name_under_experimental_gets_the_same_warning_as_core() {
    let experimental_json = lint_json(
        &project(
            "klint.yaml",
            experimental("    no-such-rule: error\n").as_bytes(),
        )
        .root,
    );
    let core_json = lint_json(
        &project(
            "klint.yaml",
            b"include: [\"src\"]\nrules:\n  core:\n    no-such-rule: error\n",
        )
        .root,
    );
    let wanted = "Unknown rule \"no-such-rule\" is ignored — check its name.";
    assert_eq!(warnings_naming(&core_json, "no-such-rule"), vec![wanted]);
    assert_eq!(
        warnings_naming(&experimental_json, "no-such-rule"),
        vec![wanted]
    );
}

#[test]
fn experimental_is_no_longer_a_reserved_key() {
    let config = experimental("    no-date-equality: error\n");
    let json = lint_json(&with_source(&config, DATES).root);
    assert!(config_warnings(&json).is_empty(), "{json}");
    assert!(rule_warnings(&json).is_empty(), "{json}");
}

#[test]
fn a_config_with_only_experimental_is_v2_for_check_and_migrate() {
    let config = experimental("    no-date-equality: error\n    no-floating-promise: warn\n");
    let p = with_source(&config, BOTH);
    let check = klint(&p.root, &["migrate", "--check"]);
    assert_eq!(check.status, 0, "{}", check.all());
    let json = lint_json(&p.root);
    assert!(
        !config_warnings(&json)
            .iter()
            .any(|w| w["message"].as_str().expect("m").contains("v1 format")),
        "{json}"
    );
}

#[test]
fn migrate_leaves_the_experimental_group_untouched_every_time() {
    let config = "include: [\"src\"]\nrules:\n  recommended: true\n  experimental:\n    no-date-equality:\n      severity: warn\n      include: [\"src/**\"]\n    no-floating-promise: off\n";
    let p = with_source(config, BOTH);

    for _ in 0..2 {
        let ran = klint(&p.root, &["migrate"]);
        assert_eq!(ran.status, 0, "{}", ran.all());
        assert_eq!(
            read(p.root.join("klint.yaml")).expect("read"),
            config.as_bytes()
        );
    }
}
