mod blank_lines_support;
mod common;

use blank_lines_support::fixtures::{JS_BEFORE_FILE, LANGS, TS_AFTER, TS_BEFORE};
use blank_lines_support::*;

fn single(file: &str, text: &str, rules: &str) -> Fixture {
    fixture("blank-report", &config_for(rules), &[(file, text)])
}

#[test]
fn before_rule_reports_every_target_kind_in_every_language() {
    for lang in &LANGS {
        let fx = single(
            lang.file,
            lang.before_report,
            &format!("  {BEFORE}: error\n"),
        );
        let json = lint(&fx);
        assert_eq!(
            lines_of(&json, BEFORE, lang.file),
            lang.before_lines,
            "{}",
            lang.file
        );
    }
}

#[test]
fn before_rule_runs_on_js_files() {
    let fx = single(JS_BEFORE_FILE, TS_BEFORE, &format!("  {BEFORE}: error\n"));
    let json = lint(&fx);
    assert_eq!(
        lines_of(&json, BEFORE, JS_BEFORE_FILE),
        LANGS[0].before_lines
    );
}

#[test]
fn after_rule_reports_every_block_kind_in_every_language() {
    for lang in &LANGS {
        let fx = single(lang.file, lang.after_report, &format!("  {AFTER}: error\n"));
        let json = lint(&fx);
        assert_eq!(
            lines_of(&json, AFTER, lang.file),
            lang.after_lines,
            "{}",
            lang.file
        );
    }
}

#[test]
fn after_rule_runs_on_js_files() {
    let fx = single("a.js", TS_AFTER, &format!("  {AFTER}: error\n"));
    let json = lint(&fx);
    assert_eq!(lines_of(&json, AFTER, "a.js"), LANGS[0].after_lines);
}

#[test]
fn before_rule_stays_quiet_on_every_must_not_case() {
    for lang in &LANGS {
        let fx = single(
            lang.file,
            &ws(lang.before_clean),
            &format!("  {BEFORE}: error\n"),
        );
        let json = lint(&fx);
        assert_eq!(
            lines_of(&json, BEFORE, lang.file),
            Vec::<u64>::new(),
            "{}",
            lang.file
        );
    }
}

#[test]
fn after_rule_stays_quiet_on_every_must_not_case() {
    for lang in &LANGS {
        let fx = single(
            lang.file,
            &ws(lang.after_clean),
            &format!("  {AFTER}: error\n"),
        );
        let json = lint(&fx);
        assert_eq!(
            lines_of(&json, AFTER, lang.file),
            Vec::<u64>::new(),
            "{}",
            lang.file
        );
    }
}

#[test]
fn messages_name_the_keyword_as_written() {
    let cases: [(&str, &str, &str, &str); 4] = [
        (
            "a.py",
            LANGS[1].before_report,
            BEFORE,
            "Missing blank line before `match` — separate it from the statement above.",
        ),
        (
            "a.cs",
            LANGS[4].before_report,
            BEFORE,
            "Missing blank line before `foreach` — separate it from the statement above.",
        ),
        (
            "a.swift",
            LANGS[2].before_report,
            BEFORE,
            "Missing blank line before `guard` — separate it from the statement above.",
        ),
        (
            "a.rs",
            LANGS[3].before_report,
            BEFORE,
            "Missing blank line before `loop` — separate it from the statement above.",
        ),
    ];

    for (file, text, rule, expected) in cases {
        let fx = single(file, text, &format!("  {rule}: error\n"));
        assert!(
            messages(&lint(&fx), rule).contains(&expected.to_string()),
            "{file}"
        );
    }

    let fx = single("a.ts", TS_AFTER, &format!("  {AFTER}: error\n"));
    assert!(messages(&lint(&fx), AFTER).contains(
        &"Missing blank line after `if` block — separate it from the statement below.".to_string()
    ));
}

#[test]
fn rules_accept_a_bare_severity_and_an_options_object() {
    let bare = single(
        "a.ts",
        LANGS[0].before_report,
        &format!("  {BEFORE}: warn\n"),
    );
    assert_eq!(severities(&lint(&bare), BEFORE), vec!["warn"; 9]);
    let object = single(
        "a.ts",
        LANGS[0].after_report,
        &format!("  {AFTER}:\n    severity: warn\n"),
    );
    assert_eq!(severities(&lint(&object), AFTER), vec!["warn"; 7]);
}

#[test]
fn rules_are_opt_in() {
    let fx = fixture(
        "blank-optin",
        "include: [\"src\"]\nplugins: [sonar]\nrules: {}\n",
        &[
            ("a.ts", LANGS[0].before_report),
            ("b.ts", LANGS[0].after_report),
        ],
    );
    let json = lint(&fx);
    assert!(hits(&json, BEFORE).is_empty() && hits(&json, AFTER).is_empty());
}

#[test]
fn statements_narrows_the_targeted_kinds() {
    let before = single(
        "a.ts",
        LANGS[0].before_report,
        &format!("  {BEFORE}:\n    severity: error\n    statements: [return]\n"),
    );
    assert_eq!(lines_of(&lint(&before), BEFORE, "a.ts"), vec![19, 28]);
    let after = single(
        "a.ts",
        LANGS[0].after_report,
        &format!("  {AFTER}:\n    severity: error\n    statements: [for]\n"),
    );
    assert_eq!(lines_of(&lint(&after), AFTER, "a.ts"), vec![10, 13]);
}

#[test]
fn empty_statements_reports_nothing() {
    let rules = format!("  {BEFORE}:\n    statements: []\n  {AFTER}:\n    statements: []\n");
    let fx = fixture(
        "blank-empty",
        &config_for(&rules),
        &[
            ("a.ts", LANGS[0].before_report),
            ("b.ts", LANGS[0].after_report),
        ],
    );
    let json = lint(&fx);
    assert!(hits(&json, BEFORE).is_empty() && hits(&json, AFTER).is_empty());
}

#[test]
fn include_scopes_each_rule() {
    let rules = format!(
        "  {BEFORE}:\n    include: [\"src/keep/**\", \"!src/keep/skip/**\"]\n  {AFTER}:\n    include: [\"src/other/**\"]\n"
    );
    let files = [
        ("keep/a.ts", LANGS[0].before_report),
        ("keep/skip/a.ts", LANGS[0].before_report),
        ("other/a.ts", LANGS[0].after_report),
        ("keep/b.ts", LANGS[0].after_report),
    ];
    let json = lint(&fixture("blank-include", &config_for(&rules), &files));
    assert_eq!(lines_of(&json, BEFORE, "keep/a.ts"), LANGS[0].before_lines);
    assert!(lines_of(&json, BEFORE, "keep/skip/a.ts").is_empty());
    assert_eq!(lines_of(&json, AFTER, "other/a.ts"), LANGS[0].after_lines);
    assert!(lines_of(&json, AFTER, "keep/b.ts").is_empty());
    assert!(lines_of(&json, BEFORE, "other/a.ts").is_empty());
}

#[test]
fn existing_rules_stay_javascript_only() {
    let rules = "  sonar/prefer-at: error\n".to_string();
    let files = [
        ("a.py", "x = arr[arr.length - 1]\n"),
        ("a.ts", "const x = arr[arr.length - 1];\n"),
    ];
    let json = lint(&fixture("blank-js-only", &config_for(&rules), &files));
    assert_eq!(
        hits(&json, "sonar/prefer-at"),
        vec![("src/a.ts".to_string(), 1)]
    );
}

#[test]
fn match_is_an_alias_of_switch_for_both_rules() {
    for rule in [BEFORE, AFTER] {
        let run = |statements: &str| {
            let rules = format!("  {rule}:\n    statements: {statements}\n");
            let files: Vec<(&str, &str)> = LANGS
                .iter()
                .map(|l| {
                    (
                        l.file,
                        if rule == BEFORE {
                            l.before_report
                        } else {
                            l.after_report
                        },
                    )
                })
                .collect();
            let json = lint(&fixture("blank-alias", &config_for(&rules), &files));
            (hits(&json, rule), hits(&json, "klint/config"))
        };
        let (switch_hits, _) = run("[switch]");
        let (match_hits, warnings) = run("[match]");
        let (both_hits, _) = run("[match, switch]");
        assert!(!switch_hits.is_empty(), "{rule}");
        assert_eq!(match_hits, switch_hits, "{rule}");
        assert_eq!(both_hits, switch_hits, "{rule}");
        assert_eq!(
            warnings,
            vec![("klint.yaml".to_string(), 1)],
            "{rule}: {warnings:?}"
        );
        assert!(
            match_hits.iter().any(|(f, _)| f.ends_with("a.py")),
            "{rule}"
        );
        assert!(
            match_hits.iter().any(|(f, _)| f.ends_with("a.rs")),
            "{rule}"
        );
    }
}

#[test]
fn setup_assignment_below_a_block_is_still_reported_by_the_after_rule() {
    let fx = single(
        "a.ts",
        "if (a) {\n  b();\n}\nx = f();\n",
        &format!("  {AFTER}: error\n"),
    );
    assert_eq!(lines_of(&lint(&fx), AFTER, "a.ts"), vec![3]);
    let opts = format!("  {AFTER}:\n    allowSetup: true\n");
    let fx = single("a.ts", "if (a) {\n  b();\n}\nx = f();\n", &opts);
    assert_eq!(lines_of(&lint(&fx), AFTER, "a.ts"), vec![3]);
}

#[test]
fn return_after_a_block_is_never_reported_by_the_after_rule() {
    let fx = single(
        "a.ts",
        "function f(a) {\n  if (a) {\n    b();\n  }\n\n  return 1;\n  foo();\n}\n",
        &format!("  {AFTER}: error\n"),
    );
    assert!(lines_of(&lint(&fx), AFTER, "a.ts").is_empty());
}

#[test]
fn expressions_are_not_statements() {
    let rust = "fn main() {\n    foo();\n    let x = if a {\n        1\n    } else {\n        2\n    };\n    let r = match a {\n        1 => 2,\n        _ => 3,\n    };\n    bar(match a {\n        _ => 1,\n    });\n    baz();\n}\n\nfn g(a: i32) -> i32 {\n    foo();\n    match a {\n        1 => 2,\n        _ => 3,\n    }\n}\n";
    let swift = "foo()\nlet x = if a {\n    1\n} else {\n    2\n}\nbar()\n";
    let cs = "var s = Foo();\nvar r = a switch {\n    1 => 2,\n    _ => 3,\n};\nBar(a switch { _ => 1 });\nBaz();\n";
    let fx = fixture(
        "blank-expr",
        &both(),
        &[("a.rs", rust), ("a.swift", swift), ("a.cs", cs)],
    );
    let json = lint(&fx);
    assert_eq!(lines_of(&json, BEFORE, "a.rs"), vec![20]);
    assert!(lines_of(&json, AFTER, "a.rs").is_empty());

    for file in ["a.swift", "a.cs"] {
        assert!(lines_of(&json, BEFORE, file).is_empty(), "{file}");
        assert!(lines_of(&json, AFTER, file).is_empty(), "{file}");
    }
}

#[test]
fn files_with_a_syntax_error_get_no_report() {
    let broken = [
        (
            "a.ts",
            "foo();\nif (a) {\n  b();\n}\nwhile (c {\n  d();\n}\n",
        ),
        ("a.py", "foo()\nif a:\n    b()\nwhile (:\n    d()\n"),
        (
            "a.swift",
            "foo()\nif a {\n    b()\n}\nwhile ( {\n    d()\n}\n",
        ),
        (
            "a.rs",
            "fn main() {\n    foo();\n    if a {\n        b();\n    }\n    while ( {\n        d();\n    }\n}\n",
        ),
        (
            "a.cs",
            "Foo();\nif (a) {\n    B();\n}\nwhile ( {\n    D();\n}\n",
        ),
    ];
    let fx = fixture("blank-broken", &both(), &broken);
    let json = lint(&fx);
    assert!(hits(&json, BEFORE).is_empty() && hits(&json, AFTER).is_empty());
    fix(&fx);

    for (file, original) in broken {
        assert_eq!(text(&fx, file), original, "{file}");
    }
}
