mod blank_lines_support;
mod common;

use blank_lines_support::*;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const V1_WARNING: &str =
    "This config uses the v1 format — run `klint migrate` to upgrade it in place.";

fn config_messages(json: &Value) -> Vec<String> {
    messages(json, "klint/config")
}

#[test]
fn unknown_statement_keyword_warns_once_and_the_rest_still_apply() {
    let rules = format!("  {BEFORE}:\n    statements: [if, bogus]\n");
    let source = "foo();\nif (a) {\n  b();\n}\nwhile (c) {\n  d();\n}\n";
    let json = lint(&fixture(
        "blank-kw",
        &config_for(&rules),
        &[("a.ts", source)],
    ));
    let warnings = config_messages(&json);
    assert_eq!(warnings.len(), 2, "{warnings:?}");
    assert!(
        warnings[0].contains(BEFORE) && warnings[0].contains("bogus"),
        "{warnings:?}"
    );
    assert_eq!(lines_of(&json, BEFORE, "a.ts"), vec![2]);
}

#[test]
fn return_in_the_after_rule_warns_once_and_the_rest_still_apply() {
    let rules = format!("  {AFTER}:\n    statements: [return, for]\n");
    let source = "for (const x of y) {\n  b();\n}\nfoo();\n";
    let json = lint(&fixture(
        "blank-ret",
        &config_for(&rules),
        &[("a.ts", source)],
    ));
    let warnings = config_messages(&json);
    assert_eq!(warnings.len(), 2, "{warnings:?}");
    assert!(
        warnings[0].contains(AFTER) && warnings[0].contains("return"),
        "{warnings:?}"
    );
    assert_eq!(lines_of(&json, AFTER, "a.ts"), vec![3]);
}

#[test]
fn match_keyword_and_return_in_the_before_rule_raise_no_warning() {
    let rules = format!(
        "  {BEFORE}:\n    statements: [match, switch, return]\n  {AFTER}:\n    statements: [match, switch]\n"
    );
    let json = lint(&fixture(
        "blank-nowarn",
        &config_for(&rules),
        &[("a.ts", "foo();\n")],
    ));
    assert_eq!(config_messages(&json), vec![V1_WARNING], "{json}");
}

#[test]
fn bare_and_object_forms_raise_no_config_warning() {
    let rules = format!(
        "  {BEFORE}:\n    severity: warn\n    include: [\"src/**\"]\n    allowSetup: true\n  {AFTER}: warn\n"
    );
    let json = lint(&fixture(
        "blank-forms",
        &config_for(&rules),
        &[("a.ts", "foo();\n")],
    ));
    assert_eq!(config_messages(&json), vec![V1_WARNING], "{json}");
}

fn committed(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn rule_schema(schema: &Value, rule: &str) -> Value {
    match schema {
        Value::Object(map) => map.get(rule).cloned().or_else(|| {
            map.values()
                .map(|v| rule_schema(v, rule))
                .find(|v| !v.is_null())
        }),
        Value::Array(items) => items
            .iter()
            .map(|v| rule_schema(v, rule))
            .find(|v| !v.is_null()),
        _ => None,
    }
    .unwrap_or(Value::Null)
}

fn options_properties(rule: &Value) -> Value {
    rule["anyOf"]
        .as_array()
        .expect("anyOf")
        .iter()
        .find(|v| v["type"] == "object")
        .expect("options object")["properties"]
        .clone()
}

#[test]
fn committed_schema_describes_both_rules_with_their_options() {
    let schema: Value = serde_json::from_str(&committed("klint.schema.json")).expect("schema json");
    let before = rule_schema(&schema, BEFORE);
    let after = rule_schema(&schema, AFTER);
    for rule in [&before, &after] {
        assert!(rule["description"].as_str().is_some_and(|d| !d.is_empty()));
        assert!(rule["examples"].as_array().is_some_and(|e| !e.is_empty()));
    }
    let before_props = options_properties(&before);
    let after_props = options_properties(&after);
    assert!(
        before_props["statements"]
            .to_string()
            .contains("\"return\"")
    );
    assert!(before_props["statements"].to_string().contains("\"match\""));
    assert_eq!(before_props["allowSetup"]["type"], "boolean");
    assert!(after_props["statements"].to_string().contains("\"match\""));
    assert!(!after_props["statements"].to_string().contains("\"return\""));
    assert!(after_props.get("allowSetup").is_none());
    let yaml = committed("klint.schema.yaml");
    assert!(yaml.contains(BEFORE) && yaml.contains(AFTER) && yaml.contains("allowSetup"));
}

#[test]
fn installed_skill_documents_both_rules() {
    let fx = fixture("blank-skill", "include: [\".\"]\n", &[]);
    let status = Command::new(env!("CARGO_BIN_EXE_klint-rs"))
        .arg("install-skill")
        .current_dir(&fx.root)
        .output()
        .expect("klint-rs runs");
    assert_eq!(status.status.code(), Some(0));
    let skill = fs::read_to_string(fx.root.join(".agents/skills/klint-rules/SKILL.md"))
        .expect("installed skill");
    assert!(skill.contains(BEFORE) && skill.contains(AFTER));
}
