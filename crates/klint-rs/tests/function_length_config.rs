mod common;
mod complexity_support;
mod function_length_support;

use complexity_support::{bytes, fix, fixture, lint, messages, violations};
use function_length_support::*;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

const CONFIG: &str = "klint/config";
const V1_WARNING: &str =
    "This config uses the v1 format — run `klint migrate` to upgrade it in place.";

fn error_config() -> String {
    format!("include: [\"src\"]\nrules:\n  {RULE}: error\n")
}

#[test]
fn bare_error_reports_fifty_one_and_not_fifty_with_exact_message_and_no_fix() {
    let big = ts_function_of_lines("big", 51);
    let ok = ts_function_of_lines("ok", 50);
    let fx = fixture(
        "fl-default",
        &error_config(),
        &[("a.ts", &format!("{big}{ok}"))],
    );
    let json = lint(&fx);
    let found = violations(&json, RULE);
    assert_eq!(found.len(), 1, "{json}");
    assert_eq!(found[0]["line"], 1);
    assert_eq!(found[0]["severity"], "error");
    assert!(found[0]["fix"].is_null());
    assert_eq!(found[0]["message"], max_message("big", 51, 50));
}

#[test]
fn max_option_reports_strictly_greater_counts() {
    let four = ts_function_of_lines("four", 4);
    let three = ts_function_of_lines("three", 3);
    let got = lint_messages("    max: 3\n", &[("a.ts", &format!("{four}{three}"))]);
    assert_eq!(got, vec![max_message("four", 4, 3)]);
}

#[test]
fn severity_option_is_honoured() {
    let fx = fixture(
        "fl-warn",
        &config("    severity: warn\n    max: 1\n"),
        &[("a.ts", &ts_function_of_lines("f", 2))],
    );
    let json = lint(&fx);
    let found = violations(&json, RULE);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0]["severity"], "warn");
}

#[test]
fn include_limits_the_rule_to_matching_files() {
    let source = ts_function_of_lines("big", 51);
    let fx = fixture(
        "fl-include",
        &config("    include: [\"src/kept/**\"]\n"),
        &[("kept/a.ts", &source), ("other/b.ts", &source)],
    );
    assert_eq!(files_hit_for(&lint(&fx), RULE), vec!["src/kept/a.ts"]);
}

#[test]
fn skip_blank_lines_drops_blank_lines_from_the_count() {
    let source = "function f() {\n  a();\n\n  b();\n}\n";
    expect_count("a.ts", source, false, 3);
    expect_count("a.ts", source, true, 2);
}

#[test]
fn invalid_max_values_warn_once_each_and_the_rule_runs_with_fifty() {
    for value in ["0", "-1", "2.5", "\"x\""] {
        let source = ts_function_of_lines("big", 51);
        let fx = fixture(
            "fl-badmax",
            &config(&format!("    max: {value}\n")),
            &[("a.ts", &source)],
        );
        let json = lint(&fx);
        let warnings = messages(&json, CONFIG);
        assert_eq!(warnings.len(), 2, "{value}: {warnings:?}");
        let shown = value.trim_matches('"');
        assert!(
            warnings[0].contains(RULE) && warnings[0].contains(shown),
            "{value}: {warnings:?}"
        );
        assert_eq!(
            messages(&json, RULE),
            vec![max_message("big", 51, 50)],
            "{value}"
        );
    }
}

#[test]
fn invalid_skip_blank_lines_warns_once_each_and_the_rule_counts_blank_lines() {
    for value in ["\"yes\"", "1"] {
        let source = "function f() {\n  a();\n\n  b();\n}\n";
        let fx = fixture(
            "fl-badskip",
            &config(&format!("    max: 2\n    skipBlankLines: {value}\n")),
            &[("a.ts", source)],
        );
        let json = lint(&fx);
        let warnings = messages(&json, CONFIG);
        assert_eq!(warnings.len(), 2, "{value}: {warnings:?}");
        let shown = value.trim_matches('"');
        assert!(
            warnings[0].contains(RULE) && warnings[0].contains(shown),
            "{value}: {warnings:?}"
        );
        assert_eq!(
            messages(&json, RULE),
            vec![max_message("f", 3, 2)],
            "{value}"
        );
    }
}

#[test]
fn valid_options_raise_no_config_warning() {
    let fx = fixture(
        "fl-good",
        &config("    max: 2\n    skipBlankLines: true\n"),
        &[("a.ts", &ts_function_of_lines("f", 3))],
    );
    let json = lint(&fx);
    assert_eq!(messages(&json, CONFIG), vec![V1_WARNING], "{json}");
    assert_eq!(violations(&json, RULE).len(), 1);
}

const BROKEN: &[(&str, &str)] = &[
    ("bad.ts", "function f() {\n  a();\n  b();\n}\nconst x = ;\n"),
    ("bad.py", "def f():\n    a\n    b\nx = = 1\n"),
    ("bad.rs", "fn f() {\n    a();\n    b();\n}\nfn broken( {\n"),
    (
        "bad.swift",
        "func f() {\n    a()\n    b()\n}\nlet x = = 1\n",
    ),
    (
        "bad.cs",
        "class C {\n    void F() {\n        A();\n        B();\n    }\n    int = ;\n}\n",
    ),
];

const VALID: &[(&str, &str)] = &[
    ("ok.ts", "function f() {\n  a();\n  b();\n}\n"),
    ("ok.py", "def f():\n    a\n    b\n"),
    ("ok.rs", "fn f() {\n    a();\n    b();\n}\n"),
    ("ok.swift", "func f() {\n    a()\n    b()\n}\n"),
    (
        "ok.cs",
        "class C {\n    void F() {\n        A();\n        B();\n    }\n}\n",
    ),
];

#[test]
fn files_with_syntax_errors_get_no_diagnostic_while_valid_siblings_do() {
    let files: Vec<(&str, &str)> = BROKEN.iter().chain(VALID).copied().collect();
    let fx = fixture("fl-syntax", &config_max_one(false), &files);
    let mut hit = files_hit_for(&lint(&fx), RULE);
    hit.sort();
    assert_eq!(
        hit,
        vec![
            "src/ok.cs",
            "src/ok.py",
            "src/ok.rs",
            "src/ok.swift",
            "src/ok.ts"
        ]
    );
}

#[test]
fn fix_leaves_a_file_with_only_function_length_violations_untouched() {
    let source = ts_function_of_lines("big", 51);
    let fx = fixture("fl-fix", &error_config(), &[("a.ts", &source)]);
    assert_eq!(violations(&lint(&fx), RULE).len(), 1);
    let before = bytes(&fx, "a.ts");
    fix(&fx);
    assert_eq!(bytes(&fx, "a.ts"), before);
    let second = fix(&fx);
    assert_eq!(bytes(&fx, "a.ts"), before);
    assert!(second.contains("applied 0 fix(es)"), "{second}");
    assert_eq!(violations(&lint(&fx), RULE).len(), 1);
}

#[test]
fn running_twice_reports_the_same_findings() {
    let fx = fixture(
        "fl-twice",
        &error_config(),
        &[("a.ts", &ts_function_of_lines("big", 51))],
    );
    let first = rows(&lint(&fx));
    assert_eq!(first.len(), 1, "{first:?}");
    assert_eq!(first, rows(&lint(&fx)));
}

#[test]
fn diagnostics_are_ordered_by_line() {
    let source = format!(
        "{}{}{}",
        ts_function_of_lines("one", 3),
        ts_function_of_lines("two", 3),
        ts_function_of_lines("three", 3)
    );
    let got: Vec<u64> = rows_for("a.ts", &source, false)
        .iter()
        .map(|r| r.0)
        .collect();
    assert_eq!(got, vec![1, 6, 11]);
}

fn committed(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn find_key(value: &Value, key: &str) -> Value {
    match value {
        Value::Object(map) => map.get(key).cloned().or_else(|| {
            map.values()
                .map(|v| find_key(v, key))
                .find(|v| !v.is_null())
        }),
        Value::Array(items) => items
            .iter()
            .map(|v| find_key(v, key))
            .find(|v| !v.is_null()),
        _ => None,
    }
    .unwrap_or(Value::Null)
}

#[test]
fn committed_schema_describes_the_rule_options() {
    let schema: Value = serde_json::from_str(&committed("klint.schema.json")).expect("schema json");
    let rule = find_key(&schema, RULE);
    assert!(rule["description"].as_str().is_some_and(|d| !d.is_empty()));
    assert!(rule["examples"].as_array().is_some_and(|e| !e.is_empty()));
    let options = rule["anyOf"]
        .as_array()
        .expect("anyOf")
        .iter()
        .find(|v| v["type"] == "object")
        .expect("options object");
    assert_eq!(options["additionalProperties"], false);
    let props = &options["properties"];
    assert!(props.get("include").is_some());
    assert_eq!(props["warn"]["type"], "integer");
    assert_eq!(props["warn"]["minimum"], 1);
    assert_eq!(props["error"]["type"], "integer");
    assert_eq!(props["error"]["minimum"], 1);
    assert!(props.get("max").is_none() && props.get("severity").is_none());
    assert_eq!(props["skipBlankLines"]["type"], "boolean");
    assert_eq!(props["skipBlankLines"]["default"], false);
    let yaml = committed("klint.schema.yaml");
    assert!(yaml.contains(RULE) && yaml.contains("skipBlankLines"));
}

#[test]
fn skill_and_readme_document_the_rule() {
    let skill = committed("skill/klint-rules/SKILL.md");
    assert!(skill.contains(RULE) && skill.contains("max") && skill.contains("skipBlankLines"));
    let readme = committed("README.md");
    assert!(readme.contains(RULE));
    assert!(readme.contains("skipBlankLines"));
    assert!(readme.contains("Biome"));
    assert!(readme.contains("accessor") || readme.contains("getter"));
}
