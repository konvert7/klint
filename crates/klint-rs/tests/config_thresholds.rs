mod common;
mod migrate_support;

use migrate_support::*;
use serde_json::Value;
use std::fs::{create_dir_all, write};

const CC: &str = "cognitive-complexity";
const FL: &str = "function-length";
const TREE: &str = "include: [\"src\", \"scripts\", \"tools\"]\n";

fn cc_function(name: &str, score: usize) -> String {
    format!(
        "function {name}(a) {{\n{}}}\n",
        "  if (a) {\n    g();\n  }\n".repeat(score)
    )
}

fn fl_function(name: &str, lines: usize) -> String {
    format!("function {name}() {{\n{}}}\n", "  a();\n".repeat(lines))
}

fn config_v2(rule: &str, value: &str) -> String {
    format!("{TREE}rules:\n  core:\n    {rule}:{value}\n")
}

fn setup(config: &str, files: &[(&str, String)]) -> Project {
    let p = project("klint.yaml", config.as_bytes());

    for (path, text) in files {
        let target = p.root.join(path);
        create_dir_all(target.parent().expect("parent")).expect("dirs");
        write(target, text).expect("write source");
    }

    p
}

fn hits(json: &Value, rule: &str) -> Vec<(String, String, String)> {
    violations(json)
        .iter()
        .filter(|v| v["rule"] == rule)
        .map(|v| {
            (
                v["file"].as_str().expect("file").to_string(),
                v["severity"].as_str().expect("sev").to_string(),
                v["message"].as_str().expect("msg").to_string(),
            )
        })
        .collect()
}

fn fl_msg(name: &str, count: usize, max: usize) -> String {
    format!("Function `{name}` has {count} lines (max {max}) — split it into smaller functions.")
}

fn cc_msg(name: &str, score: usize, max: usize) -> String {
    format!(
        "Function `{name}` has cognitive complexity {score} (max {max}) — split it into smaller functions."
    )
}

fn sev_msgs(json: &Value, rule: &str) -> Vec<(String, String)> {
    hits(json, rule)
        .into_iter()
        .map(|(_, s, m)| (s, m))
        .collect()
}

#[test]
fn function_length_warn_and_error_give_one_diagnostic_at_the_highest_level() {
    let source: String = [60, 61, 80, 81]
        .iter()
        .map(|n| fl_function(&format!("f{n}"), *n))
        .collect();
    let config = config_v2(FL, " {warn: 60, error: 80, skipBlankLines: true}");
    let json = lint_json(&setup(&config, &[("src/a.ts", source)]).root);

    assert_eq!(
        sev_msgs(&json, FL),
        vec![
            ("warn".to_string(), fl_msg("f61", 61, 60)),
            ("warn".to_string(), fl_msg("f80", 80, 60)),
            ("error".to_string(), fl_msg("f81", 81, 80)),
        ]
    );
    assert!(config_warnings(&json).is_empty(), "{json}");
}

#[test]
fn complexity_warn_and_error_give_one_diagnostic_at_the_highest_level() {
    let source: String = [10, 11, 20, 21]
        .iter()
        .map(|n| cc_function(&format!("f{n}"), *n))
        .collect();
    let config = config_v2(CC, " {warn: 10, error: 20}");
    let json = lint_json(&setup(&config, &[("src/a.ts", source)]).root);

    assert_eq!(
        sev_msgs(&json, CC),
        vec![
            ("warn".to_string(), cc_msg("f11", 11, 10)),
            ("warn".to_string(), cc_msg("f20", 20, 10)),
            ("error".to_string(), cc_msg("f21", 21, 20)),
        ]
    );
    assert!(config_warnings(&json).is_empty(), "{json}");
}

#[test]
fn warn_alone_and_error_alone_each_work_on_both_rules() {
    let fl = format!("{}{}", fl_function("at", 5), fl_function("over", 6));
    let cc = format!("{}{}", cc_function("at", 5), cc_function("over", 6));
    let fl_files = [("src/a.ts", fl)];
    let cc_files = [("src/a.ts", cc)];

    for (level, other) in [("warn", "error"), ("error", "warn")] {
        let fl_config = config_v2(FL, &format!(" {{{level}: 5}}"));
        let json = lint_json(&setup(&fl_config, &fl_files).root);
        assert_eq!(
            sev_msgs(&json, FL),
            vec![(level.to_string(), fl_msg("over", 6, 5))],
            "{other}"
        );
        let cc_config = config_v2(CC, &format!(" {{{level}: 5}}"));
        let json = lint_json(&setup(&cc_config, &cc_files).root);
        assert_eq!(
            sev_msgs(&json, CC),
            vec![(level.to_string(), cc_msg("over", 6, 5))]
        );
    }
}

#[test]
fn error_below_warn_warns_once_on_the_rule_line_and_runs_with_error_only() {
    let config = config_v2(CC, " {warn: 20, error: 10}");
    let source = format!("{}{}", cc_function("mid", 15), cc_function("low", 10));
    let json = lint_json(&setup(&config, &[("src/a.ts", source)]).root);

    let warnings = config_warnings(&json);
    assert_eq!(warnings.len(), 1, "{json}");
    assert_eq!(warnings[0]["line"], 4);
    assert_eq!(warnings[0]["severity"], "warn");
    let text = warnings[0]["message"].as_str().expect("m");
    assert!(text.contains("20") && text.contains("10"), "{text}");
    assert_eq!(
        sev_msgs(&json, CC),
        vec![("error".to_string(), cc_msg("mid", 15, 10))]
    );
}

#[test]
fn error_equal_to_warn_is_valid_and_behaves_as_error_only() {
    let config = config_v2(FL, " {warn: 5, error: 5}");
    let json = lint_json(&setup(&config, &[("src/a.ts", fl_function("f", 6))]).root);

    assert!(config_warnings(&json).is_empty(), "{json}");
    assert_eq!(
        sev_msgs(&json, FL),
        vec![("error".to_string(), fl_msg("f", 6, 5))]
    );
}

fn two_entries(scripts_first: bool) -> String {
    let all = "      - {error: 15}\n";
    let scripts = "      - {warn: 10, error: 20, include: [\"scripts/**\"]}\n";
    let body = if scripts_first {
        format!("{scripts}{all}")
    } else {
        format!("{all}{scripts}")
    };
    config_v2(CC, &format!("\n{}", body.trim_end()))
}

fn sixteen_in_both_dirs(config: &str) -> Value {
    let files = [
        ("src/a.ts", cc_function("f", 16)),
        ("scripts/a.ts", cc_function("f", 16)),
    ];
    lint_json(&setup(config, &files).root)
}

#[test]
fn list_form_last_matching_entry_wins() {
    let json = sixteen_in_both_dirs(&two_entries(false));
    let got = hits(&json, CC);
    assert!(
        got.contains(&("src/a.ts".into(), "error".into(), cc_msg("f", 16, 15))),
        "{got:?}"
    );
    assert!(
        got.contains(&("scripts/a.ts".into(), "warn".into(), cc_msg("f", 16, 10))),
        "{got:?}"
    );
    assert_eq!(got.len(), 2, "{got:?}");
}

#[test]
fn reversed_list_order_makes_the_catch_all_win_for_scripts() {
    let json = sixteen_in_both_dirs(&two_entries(true));
    let got = hits(&json, CC);
    assert!(
        got.contains(&("scripts/a.ts".into(), "error".into(), cc_msg("f", 16, 15))),
        "{got:?}"
    );
    assert!(
        got.contains(&("src/a.ts".into(), "error".into(), cc_msg("f", 16, 15))),
        "{got:?}"
    );
    assert_eq!(got.len(), 2, "{got:?}");
}

#[test]
fn list_form_json_config_works_too() {
    let config = "{\"include\": [\"src\", \"scripts\"], \"rules\": {\"core\": {\"cognitive-complexity\": [{\"error\": 15}, {\"warn\": 10, \"include\": [\"scripts/**\"]}]}}}\n";
    let p = project("klint.config.json", config.as_bytes());
    create_dir_all(p.root.join("scripts")).expect("dir");
    write(p.root.join("src/a.ts"), cc_function("f", 16)).expect("w");
    write(p.root.join("scripts/a.ts"), cc_function("f", 16)).expect("w");

    let got = hits(&lint_json(&p.root), CC);

    assert!(got.contains(&("src/a.ts".into(), "error".into(), cc_msg("f", 16, 15))));
    assert!(got.contains(&("scripts/a.ts".into(), "warn".into(), cc_msg("f", 16, 10))));
}

#[test]
fn list_with_negated_include_glob_excludes_those_files() {
    let config = config_v2(
        FL,
        "\n      - {error: 5, include: [\"src/**\", \"!src/skip/**\"]}",
    );
    let files = [
        ("src/a.ts", fl_function("f", 6)),
        ("src/skip/b.ts", fl_function("g", 6)),
    ];
    let got = hits(&lint_json(&setup(&config, &files).root), FL);
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0].0, "src/a.ts");
}

#[test]
fn a_file_no_entry_matches_is_not_checked() {
    let config = config_v2(CC, "\n      - {error: 15, include: [\"src/**\"]}");
    let files = [
        ("src/a.ts", cc_function("f", 16)),
        ("tools/x.ts", cc_function("g", 16)),
    ];
    let got = hits(&lint_json(&setup(&config, &files).root), CC);
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0].0, "src/a.ts");
}

#[test]
fn an_empty_list_turns_the_rule_off() {
    for (rule, source) in [(CC, cc_function("f", 40)), (FL, fl_function("f", 200))] {
        let config = config_v2(rule, " []");
        let json = lint_json(&setup(&config, &[("src/a.ts", source)]).root);
        assert!(hits(&json, rule).is_empty(), "{rule}: {json}");
        assert!(config_warnings(&json).is_empty(), "{rule}: {json}");
    }
}

#[test]
fn bare_error_warn_and_off_keep_their_defaults() {
    let source = format!(
        "{}{}{}{}",
        cc_function("c15", 15),
        cc_function("c16", 16),
        fl_function("l50", 50),
        fl_function("l51", 51)
    );
    let files = [("src/a.ts", source)];
    let run = |value: &str| {
        let config = format!("{TREE}rules:\n  core:\n    {CC}: {value}\n    {FL}: {value}\n");
        lint_json(&setup(&config, &files).root)
    };

    for sev in ["error", "warn"] {
        let json = run(sev);
        assert_eq!(
            sev_msgs(&json, CC),
            vec![(sev.to_string(), cc_msg("c16", 16, 15))]
        );
        assert_eq!(
            sev_msgs(&json, FL),
            vec![(sev.to_string(), fl_msg("l51", 51, 50))]
        );
        assert!(config_warnings(&json).is_empty(), "{json}");
    }

    let json = run("off");
    assert!(hits(&json, CC).is_empty() && hits(&json, FL).is_empty());
}

const INVALID_ENTRIES: [(&str, &str); 7] = [
    ("neither warn nor error", "{include: [\"src/**\"]}"),
    ("non-integer limit", "{error: 1.5}"),
    ("zero limit", "{error: 0}"),
    ("negative limit", "{warn: -3}"),
    ("string limit", "{error: \"x\"}"),
    ("unknown key", "{error: 5, bogus: 1}"),
    ("element not an object", "7"),
];

#[test]
fn each_invalid_list_entry_warns_once_and_the_valid_entries_still_apply() {
    for (kind, bad) in INVALID_ENTRIES {
        for (rule, big, limit) in [
            (CC, cc_function("f", 16), 15),
            (FL, fl_function("f", 16), 15),
        ] {
            let config = config_v2(
                rule,
                &format!("\n      - {bad}\n      - {{error: {limit}}}"),
            );
            let json = lint_json(&setup(&config, &[("src/a.ts", big)]).root);

            let warnings = config_warnings(&json);
            assert_eq!(warnings.len(), 1, "{kind} on {rule}: {json}");
            assert_eq!(warnings[0]["severity"], "warn", "{kind} on {rule}");
            assert_eq!(hits(&json, rule).len(), 1, "{kind} on {rule}: {json}");
        }
    }
}

#[test]
fn an_invalid_lone_object_warns_once_and_is_ignored() {
    for (kind, bad) in INVALID_ENTRIES.iter().take(6) {
        let config = config_v2(CC, &format!(" {bad}"));
        let json = lint_json(&setup(&config, &[("src/a.ts", cc_function("f", 40))]).root);

        assert_eq!(config_warnings(&json).len(), 1, "{kind}: {json}");
        assert!(hits(&json, CC).is_empty(), "{kind}: {json}");
    }
}

#[test]
fn a_list_entry_mixing_max_with_error_applies_error_and_warns_once() {
    let config = config_v2(CC, "\n      - {max: 3, error: 15}");
    let json = lint_json(&setup(&config, &[("src/a.ts", cc_function("f", 16))]).root);

    let warnings = config_warnings(&json);
    assert_eq!(warnings.len(), 1, "{json}");
    let message = warnings[0]["message"].as_str().expect("message");
    assert!(
        message.contains("entry 1 mixes max/severity with warn/error"),
        "{message}"
    );
    assert_eq!(
        sev_msgs(&json, CC),
        vec![("error".to_string(), cc_msg("f", 16, 15))]
    );
}

const OLD_WARNING_CC: &str = "`max` and `severity` on cognitive-complexity are the old form — run `klint migrate` to upgrade it in place.";

#[test]
fn old_form_in_a_v2_config_behaves_as_before_plus_one_warning_on_the_rule_line() {
    let config = config_v2(CC, "\n      max: 10\n      severity: warn");
    let source = format!("{}{}", cc_function("hi", 11), cc_function("at", 10));
    let json = lint_json(&setup(&config, &[("src/a.ts", source)]).root);

    assert_eq!(
        sev_msgs(&json, CC),
        vec![("warn".to_string(), cc_msg("hi", 11, 10))]
    );
    let warnings = config_warnings(&json);
    assert_eq!(warnings.len(), 1, "{json}");
    assert_eq!(warnings[0]["message"], OLD_WARNING_CC);
    assert_eq!(warnings[0]["line"], 4);
}

#[test]
fn old_form_warning_names_the_function_length_rule_too() {
    let config = config_v2(FL, " {max: 5}");
    let json = lint_json(&setup(&config, &[("src/a.ts", fl_function("f", 6))]).root);

    let warnings = config_warnings(&json);
    assert_eq!(warnings.len(), 1, "{json}");
    assert_eq!(
        warnings[0]["message"],
        "`max` and `severity` on function-length are the old form — run `klint migrate` to upgrade it in place."
    );
    assert_eq!(
        sev_msgs(&json, FL),
        vec![("error".to_string(), fl_msg("f", 6, 5))]
    );
}

#[test]
fn old_form_in_a_v1_config_adds_no_extra_warning() {
    let config = format!("{TREE}rules:\n  {CC}:\n    max: 10\n    severity: warn\n");
    let json = lint_json(&setup(&config, &[("src/a.ts", cc_function("f", 11))]).root);

    let warnings = config_warnings(&json);
    assert_eq!(warnings.len(), 1, "{json}");
    assert!(
        warnings[0]["message"]
            .as_str()
            .expect("m")
            .contains("v1 format")
    );
    assert_eq!(
        sev_msgs(&json, CC),
        vec![("warn".to_string(), cc_msg("f", 11, 10))]
    );
}

#[test]
fn mixing_old_and_new_keys_warns_once_and_the_new_keys_apply() {
    let config = config_v2(CC, " {max: 3, severity: warn, error: 10}");
    let source = format!("{}{}", cc_function("mid", 5), cc_function("big", 11));
    let json = lint_json(&setup(&config, &[("src/a.ts", source)]).root);

    assert_eq!(config_warnings(&json).len(), 1, "{json}");
    assert_eq!(
        sev_msgs(&json, CC),
        vec![("error".to_string(), cc_msg("big", 11, 10))]
    );
}
