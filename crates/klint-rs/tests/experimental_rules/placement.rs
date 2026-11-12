use crate::migrate_support::*;
use crate::support::*;

const TYPE_CHECKER_WARNING: &str = "no-misused-promises needs a TypeScript type checker, which klint's Rust engine does not have, so it is ignored — Biome's nursery/noMisusedPromises covers it.";

#[test]
fn no_string_match_is_not_an_experimental_rule() {
    let warnings = warnings_for(
        &experimental("    no-string-match: error\n"),
        "no-string-match",
    );
    assert_eq!(
        warnings,
        [misgrouped_warning(
            "no-string-match",
            "core",
            "experimental"
        )]
    );
}

#[test]
fn no_misused_promises_is_not_an_experimental_rule() {
    let warnings = warnings_for(
        &experimental("    no-misused-promises: error\n"),
        "no-misused-promises",
    );
    assert_eq!(warnings, [TYPE_CHECKER_WARNING.to_string()]);
}

#[test]
fn core_entry_for_a_core_rule_still_runs_it() {
    let json = core_string_match_json(SOURCE);
    assert_eq!(
        findings_in(&json, "a.ts", "no-string-match").len(),
        1,
        "{json}"
    );
    assert_eq!(
        lines(&findings_in(&json, "control.ts", DATE_RULE)),
        [DATE_CONTROL_LINE],
        "{json}"
    );
}

#[test]
fn core_entry_for_an_experimental_rule_plans_nothing() {
    let json = lint_json(&with_source(&grouped("core", "no-date-equality: error"), DATES).root);
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
fn experimental_entries_both_run_in_one_pass() {
    let config = experimental("    no-floating-promise: warn\n    no-date-equality: error\n");
    let json = lint_json(&with_source(&config, BOTH).root);
    assert_eq!(lines(&findings_of(&json, DATE_RULE)), [5], "{json}");
    assert_eq!(lines(&findings_of(&json, PROMISE_RULE)), [4], "{json}");
}

#[test]
fn experimental_off_plans_nothing_and_warns_nothing() {
    let config = experimental("    no-date-equality: off\n");
    let json = lint_json(&with_source(&config, BOTH).root);
    assert!(findings_of(&json, DATE_RULE).is_empty(), "{json}");
    assert!(rule_warnings(&json).is_empty(), "{json}");
}

#[test]
fn home_group_no_date_equality_is_experimental() {
    expect_date(
        "const a = new Date(1);\nconst b = new Date(2);\nexport const same = a === b;",
        &[3],
    );
}

#[test]
fn home_group_no_string_match_is_core() {
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
fn home_group_prefer_at_is_sonar() {
    assert_eq!(
        warnings_for(&experimental("    prefer-at: error\n"), "prefer-at"),
        [misgrouped_warning("prefer-at", "sonar", "experimental")]
    );
}

#[test]
fn home_group_sonar_prefixed_name_is_sonar() {
    assert_eq!(
        warnings_for(
            &experimental("    sonar/prefer-at: error\n"),
            "sonar/prefer-at"
        ),
        [misgrouped_warning(
            "sonar/prefer-at",
            "sonar",
            "experimental"
        )]
    );
}

#[test]
fn home_group_no_misused_promises_gets_the_type_checker_warning() {
    assert_eq!(
        warnings_for(
            &experimental("    no-misused-promises: error\n"),
            "no-misused-promises"
        ),
        [TYPE_CHECKER_WARNING.to_string()]
    );
}

#[test]
fn home_group_no_such_rule_is_unknown() {
    assert_eq!(
        warnings_for(&experimental("    no-such-rule: error\n"), "no-such-rule"),
        [unknown_rule_warning("no-such-rule")]
    );
}
