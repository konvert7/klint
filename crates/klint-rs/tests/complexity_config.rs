mod common;
mod complexity_support;

use complexity_support::*;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

const CONFIG: &str = "klint/config";

fn error_config() -> String {
    format!("include: [\"src\"]\nrules:\n  {RULE}: error\n")
}

#[test]
fn bare_error_reports_sixteen_and_not_fifteen_with_exact_message_and_no_fix() {
    let big = ts_function_scoring("big", 16);
    let ok = ts_function_scoring("ok", 15);
    let fx = fixture(
        "cc-default",
        &error_config(),
        &[("a.ts", &format!("{big}{ok}"))],
    );
    let json = lint(&fx);
    let found = violations(&json, RULE);
    assert_eq!(found.len(), 1, "{json}");
    assert_eq!(found[0]["line"], 1);
    assert_eq!(found[0]["severity"], "error");
    assert!(found[0]["fix"].is_null());
    assert_eq!(
        found[0]["message"],
        "Function `big` has cognitive complexity 16 (max 15) — split it into smaller functions."
    );
}

#[test]
fn max_option_reports_strictly_greater_scores() {
    let four = ts_function_scoring("four", 4);
    let three = ts_function_scoring("three", 3);
    let fx = fixture(
        "cc-max",
        &config_max(3),
        &[("a.ts", &format!("{four}{three}"))],
    );
    let json = lint(&fx);
    assert_eq!(
        messages(&json, RULE),
        vec![
            "Function `four` has cognitive complexity 4 (max 3) — split it into smaller functions."
        ]
    );
}

#[test]
fn severity_option_is_honoured() {
    let options = "    severity: warn\n    max: 1\n";
    let fx = fixture(
        "cc-warn",
        &config_with(options),
        &[("a.ts", &ts_function_scoring("f", 2))],
    );
    let json = lint(&fx);
    let found = violations(&json, RULE);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0]["severity"], "warn");
}

#[test]
fn include_limits_the_rule_to_matching_files() {
    let source = ts_function_scoring("big", 16);
    let options = "    include: [\"src/kept/**\"]\n";
    let fx = fixture(
        "cc-include",
        &config_with(options),
        &[("kept/a.ts", &source), ("other/b.ts", &source)],
    );
    assert_eq!(files_hit(&lint(&fx)), vec!["src/kept/a.ts"]);
}

#[test]
fn invalid_max_values_warn_once_each_and_the_rule_runs_with_fifteen() {
    for value in ["0", "-1", "2.5", "\"x\""] {
        let source = ts_function_scoring("big", 16);
        let fx = fixture(
            "cc-badmax",
            &config_with(&format!("    max: {value}\n")),
            &[("a.ts", &source)],
        );
        let json = lint(&fx);
        let warnings = messages(&json, CONFIG);
        assert_eq!(warnings.len(), 1, "{value}: {warnings:?}");
        let shown = value.trim_matches('"');
        assert!(
            warnings[0].contains(RULE) && warnings[0].contains(shown),
            "{value}: {warnings:?}"
        );
        assert_eq!(
            messages(&json, RULE),
            vec![
                "Function `big` has cognitive complexity 16 (max 15) — split it into smaller functions."
            ],
            "{value}"
        );
    }
}

#[test]
fn valid_max_raises_no_config_warning() {
    let fx = fixture(
        "cc-goodmax",
        &config_max(14),
        &[("a.ts", &ts_function_scoring("f", 15))],
    );
    let json = lint(&fx);
    assert!(messages(&json, CONFIG).is_empty(), "{json}");
    assert_eq!(violations(&json, RULE).len(), 1);
}

const BROKEN: &[(&str, &str)] = &[
    (
        "bad.ts",
        "function f(a) {\n  if (a) {\n    if (a) {\n      g();\n    }\n  }\n}\nconst x = ;\n",
    ),
    (
        "bad.py",
        "def f(a):\n    if a:\n        if a:\n            g()\nx = = 1\n",
    ),
    (
        "bad.rs",
        "fn f(a: bool) {\n    if a {\n        if a {\n            g();\n        }\n    }\n}\nfn broken( {\n",
    ),
    (
        "bad.swift",
        "func f(_ a: Bool) {\n    if a {\n        if a {\n            g()\n        }\n    }\n}\nlet x = = 1\n",
    ),
    (
        "bad.cs",
        "class C {\n    void F(bool a) {\n        if (a) {\n            if (a) {\n                G();\n            }\n        }\n    }\n    int = ;\n}\n",
    ),
];

const VALID: &[(&str, &str)] = &[
    (
        "ok.ts",
        "function f(a) {\n  if (a) {\n    if (a) {\n      g();\n    }\n  }\n}\n",
    ),
    (
        "ok.py",
        "def f(a):\n    if a:\n        if a:\n            g()\n",
    ),
    (
        "ok.rs",
        "fn f(a: bool) {\n    if a {\n        if a {\n            g();\n        }\n    }\n}\n",
    ),
    (
        "ok.swift",
        "func f(_ a: Bool) {\n    if a {\n        if a {\n            g()\n        }\n    }\n}\n",
    ),
    (
        "ok.cs",
        "class C {\n    void F(bool a) {\n        if (a) {\n            if (a) {\n                G();\n            }\n        }\n    }\n}\n",
    ),
];

#[test]
fn files_with_syntax_errors_get_no_diagnostic_while_valid_siblings_do() {
    let files: Vec<(&str, &str)> = BROKEN.iter().chain(VALID).copied().collect();
    let fx = fixture("cc-syntax", &config_max(1), &files);
    let mut hit = files_hit(&lint(&fx));
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
fn fix_leaves_a_file_with_only_complexity_violations_untouched() {
    let source = ts_function_scoring("big", 16);
    let fx = fixture("cc-fix", &error_config(), &[("a.ts", &source)]);
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
    let source = ts_function_scoring("big", 16);
    let fx = fixture("cc-twice", &error_config(), &[("a.ts", &source)]);
    let first = reported(&lint(&fx));
    assert_eq!(first.len(), 1, "{first:?}");
    assert_eq!(first, reported(&lint(&fx)));
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
fn committed_schema_describes_the_rule_with_severity_include_and_max() {
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
    assert!(props.get("severity").is_some() && props.get("include").is_some());
    assert_eq!(props["max"]["type"], "integer");
    assert_eq!(props["max"]["minimum"], 1);
    assert_eq!(props["max"]["default"], 15);
    let yaml = committed("klint.schema.yaml");
    assert!(yaml.contains(RULE) && yaml.contains("max"));
}

#[test]
fn skill_and_readme_document_the_rule() {
    let skill = committed("skill/klint-rules/SKILL.md");
    assert!(skill.contains(RULE) && skill.contains("max"));
    let readme = committed("README.md");
    assert!(readme.contains(RULE));
    assert!(readme.contains("finally"));
    assert!(readme.contains("Biome"));
}
