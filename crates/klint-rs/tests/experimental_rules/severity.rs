use crate::migrate_support::*;
use crate::support::*;
use std::fs::write;

#[test]
fn experimental_object_severity_warn_with_include_reports_as_warn() {
    let config =
        experimental("    no-date-equality:\n      severity: warn\n      include: [\"src/**\"]\n");
    let json = lint_json(
        &with_source(
            &config,
            "const a = new Date(1);\nconst r = a === new Date(9);",
        )
        .root,
    );
    let found = findings_in(&json, "a.ts", DATE_RULE);
    assert_eq!(lines(&found), [2], "{json}");
    assert_eq!(found[0]["severity"], "warn");
}

#[test]
fn experimental_no_string_match_is_misgrouped_to_core() {
    assert_eq!(
        warnings_for(
            &experimental("    no-string-match: error\n"),
            "no-string-match"
        ),
        [misgrouped_warning(
            "no-string-match",
            "core",
            "experimental"
        )]
    );
}

#[test]
fn experimental_prefer_at_is_misgrouped_to_sonar() {
    assert_eq!(
        warnings_for(&experimental("    prefer-at: error\n"), "prefer-at"),
        [misgrouped_warning("prefer-at", "sonar", "experimental")]
    );
}

#[test]
fn experimental_no_such_rule_is_the_unknown_rule_advisory() {
    assert_eq!(
        warnings_for(&experimental("    no-such-rule: error\n"), "no-such-rule"),
        [unknown_rule_warning("no-such-rule")]
    );
}

#[test]
fn core_no_date_equality_is_misgrouped_and_ignored() {
    let json = lint_json(&with_source(&grouped("core", "no-date-equality: error"), BOTH).root);
    assert_eq!(
        warnings_naming(&json, DATE_RULE),
        [misgrouped_warning(
            "no-date-equality",
            "experimental",
            "core"
        )]
    );
    assert!(findings_of(&json, DATE_RULE).is_empty(), "{json}");
}

#[test]
fn sonar_no_floating_promise_is_misgrouped_and_ignored() {
    let json =
        lint_json(&with_source(&grouped("sonar", "no-floating-promise: error"), DROPPED).root);
    assert_eq!(
        warnings_naming(&json, PROMISE_RULE),
        [misgrouped_warning(
            "no-floating-promise",
            "experimental",
            "sonar"
        )]
    );
    assert!(findings_of(&json, PROMISE_RULE).is_empty(), "{json}");
}

#[test]
fn core_no_string_match_stays_in_core_and_runs() {
    let json = core_string_match_json(SOURCE);
    assert_eq!(
        findings_in(&json, "a.ts", "no-string-match").len(),
        1,
        "{json}"
    );
    assert!(rule_warnings(&json).is_empty(), "{json}");
    assert_eq!(
        lines(&findings_in(&json, "control.ts", DATE_RULE)),
        [DATE_CONTROL_LINE],
        "{json}"
    );
}

#[test]
fn experimental_list_value_is_rejected_as_an_invalid_value() {
    let p = with_source(&experimental("    no-date-equality: [\"x\"]\n"), DATES);
    let ran = klint(&p.root, &[]);
    assert_ne!(ran.status, 0, "{}", ran.all());
    assert!(ran.all().contains("failed to parse"), "{}", ran.all());
}

#[test]
fn misgrouped_message_core_date_equality_names_experimental_as_home() {
    assert_eq!(
        warnings_for(&grouped("core", "no-date-equality: error"), DATE_RULE),
        [
            "Rule \"no-date-equality\" belongs in rules.experimental, not rules.core, so this entry is ignored."
        ]
    );
}

#[test]
fn misgrouped_message_experimental_prefer_at_names_sonar_as_home() {
    assert_eq!(
        warnings_for(&experimental("    prefer-at: error\n"), "prefer-at"),
        [
            "Rule \"prefer-at\" belongs in rules.sonar, not rules.experimental, so this entry is ignored."
        ]
    );
}

#[test]
fn misgrouped_message_experimental_no_string_match_names_core_as_home() {
    assert_eq!(
        warnings_for(
            &experimental("    no-string-match: error\n"),
            "no-string-match"
        ),
        [
            "Rule \"no-string-match\" belongs in rules.core, not rules.experimental, so this entry is ignored."
        ]
    );
}

#[test]
fn severity_bogus_under_experimental_is_advised_and_ignored() {
    assert_eq!(
        warnings_for(&experimental("    no-date-equality: bogus\n"), DATE_RULE),
        [
            "Rule \"no-date-equality\" has severity \"bogus\", which is not error, warn or off, so it is ignored."
        ]
    );
}

#[test]
fn severity_warn_under_experimental_is_not_advised() {
    let json = lint_json(&with_source(&experimental("    no-date-equality: warn\n"), DATES).root);
    assert!(rule_warnings(&json).is_empty(), "{json}");
    assert_eq!(lines(&findings_of(&json, DATE_RULE)), [3], "{json}");
}

#[test]
fn severity_bogus_under_core_keeps_the_existing_advice() {
    let config = "include: [\"src\"]\nrules:\n  core:\n    no-string-match: bogus\n  experimental:\n    no-date-equality: error\n";
    let p = with_source(config, DATES);
    write(p.root.join("src/control.ts"), DATE_CONTROL).expect("write control.ts");
    let json = lint_json(&p.root);
    assert_eq!(
        warnings_naming(&json, "no-string-match"),
        [
            "Rule \"no-string-match\" has severity \"bogus\", which is not error, warn or off, so it is ignored."
        ]
    );
    assert_eq!(
        lines(&findings_in(&json, "control.ts", DATE_RULE)),
        [DATE_CONTROL_LINE],
        "{json}"
    );
}

#[test]
fn identifier_in_a_binary_comparison_is_a_reference_not_a_binding() {
    expect_date(
        "const a = new Date(1);\nconst b = new Date(2);\nconst r = a === b;",
        &[3],
    );
}

#[test]
fn declarator_name_is_a_binding_that_shadows_the_const_date() {
    expect_date(
        "const a = new Date(1);\nfunction f() { const a = x; return a === new Date(9); }",
        &[],
    );
}

#[test]
fn experimental_error_entry_for_a_date_rule_is_recorded_as_error() {
    let json = lint_json(
        &with_source(
            &experimental("    no-date-equality: error\n"),
            "const a = new Date(1);\nconst r = a === new Date(9);",
        )
        .root,
    );
    let found = findings_in(&json, "a.ts", DATE_RULE);
    assert_eq!(lines(&found), [2], "{json}");
    assert_eq!(found[0]["severity"], "error");
}
