mod blank_lines_support;
mod common;

use blank_lines_support::fix_cases::CASES;
use blank_lines_support::*;

#[test]
fn before_rule_fix_matches_expected_text_in_every_language() {
    for case in &CASES {
        fix_twice(&simple(BEFORE), case.file, case.before_in, case.before_out);
    }
}

#[test]
fn after_rule_fix_matches_expected_text_in_every_language() {
    for case in &CASES {
        fix_twice(&simple(AFTER), case.file, case.after_in, case.after_out);
    }
}

#[test]
fn both_rules_on_one_gap_insert_exactly_one_blank_line() {
    for case in &CASES {
        fix_twice(&both(), case.file, case.gap_in, case.gap_out);
    }
}

#[test]
fn both_rules_together_are_idempotent_on_the_larger_fixtures() {
    for case in &CASES {
        let fx = fixture("blank-fix-both", &both(), &[(case.file, case.after_in)]);
        fix(&fx);
        let once = source(&fx, case.file);
        assert_ne!(once, case.after_in.as_bytes(), "{}", case.file);
        let second = fix(&fx);
        assert_eq!(source(&fx, case.file), once, "{}", case.file);
        assert!(
            second.contains("applied 0 fix(es)"),
            "{}: {second}",
            case.file
        );
        let json = lint(&fx);
        assert!(hits(&json, BEFORE).is_empty() && hits(&json, AFTER).is_empty());
    }
}

#[test]
fn inserted_line_is_crlf_on_a_crlf_file() {
    let input = "foo();\r\nif (a) {\r\n  b();\r\n}\r\n";
    let expected = "foo();\r\n\r\nif (a) {\r\n  b();\r\n}\r\n";
    let fx = fixture("blank-crlf", &simple(BEFORE), &[("a.ts", input)]);
    fix(&fx);
    assert_eq!(source(&fx, "a.ts"), expected.as_bytes());
    let after_in = "if (a) {\r\n  b();\r\n}\r\nfoo();\r\n";
    let after_out = "if (a) {\r\n  b();\r\n}\r\n\r\nfoo();\r\n";
    let fx = fixture("blank-crlf-after", &simple(AFTER), &[("a.ts", after_in)]);
    fix(&fx);
    assert_eq!(source(&fx, "a.ts"), after_out.as_bytes());
}

#[test]
fn inserted_line_is_lf_on_an_lf_file() {
    let fx = fixture(
        "blank-lf",
        &simple(BEFORE),
        &[("a.ts", "foo();\nif (a) {\n  b();\n}\n")],
    );
    fix(&fx);
    assert_eq!(source(&fx, "a.ts"), b"foo();\n\nif (a) {\n  b();\n}\n");
}

#[test]
fn inserted_line_copies_the_ending_of_the_line_above_in_mixed_files() {
    let input = "foo();\nif (a) {\r\n  b();\r\n}\r\n";
    let expected = "foo();\n\nif (a) {\r\n  b();\r\n}\r\n";
    let fx = fixture("blank-mixed", &simple(BEFORE), &[("a.ts", input)]);
    fix(&fx);
    assert_eq!(source(&fx, "a.ts"), expected.as_bytes());
}

#[test]
fn whitespace_only_neighbours_count_as_blank_and_stay_untouched() {
    let input = "foo();\n  \nif (a) {\n  b();\n}\n  \nwhile (c) {\n  d();\n}\n";
    let fx = fixture("blank-ws", &both(), &[("a.ts", input)]);
    fix(&fx);
    assert_eq!(text(&fx, "a.ts"), input);
}

#[test]
fn json_report_carries_the_fix_payload() {
    let fx = fixture(
        "blank-json",
        &both(),
        &[("a.ts", "foo();\nif (a) {\n  b();\n}\n")],
    );
    let json = lint(&fx);
    let violation = &json["violations"][0];
    assert_eq!(violation["rule"], BEFORE);
    assert_eq!(violation["line"], 2);
    assert_eq!(violation["fix"]["startLine"], 2);
    assert_eq!(violation["fix"]["endLine"], 2);
    assert_eq!(violation["fix"]["replacement"], "\nif (a) {");
}
