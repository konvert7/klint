use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;

fn committed() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../klint.schema.json");
    serde_json::from_str(&fs::read_to_string(path).expect("read schema")).expect("json")
}

fn type_ok(kind: &str, value: &Value) -> bool {
    match kind {
        "string" => value.is_string(),
        "boolean" => value.is_boolean(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        "integer" => value.is_i64() || value.is_u64(),
        "number" => value.is_number(),
        _ => true,
    }
}

fn type_valid(schema: &Value, value: &Value) -> bool {
    match &schema["type"] {
        Value::String(kind) => type_ok(kind, value),
        Value::Array(kinds) => kinds
            .iter()
            .any(|k| type_ok(k.as_str().unwrap_or(""), value)),
        _ => true,
    }
}

fn members_valid(schema: &Value, value: &Value) -> bool {
    let Some(map) = value.as_object() else {
        return true;
    };
    let props = schema["properties"].as_object();
    let required = schema["required"].as_array().into_iter().flatten();
    let present = required
        .filter_map(Value::as_str)
        .all(|r| map.contains_key(r));
    present
        && map
            .iter()
            .all(|(key, v)| match props.and_then(|p| p.get(key)) {
                Some(sub) => valid(sub, v),
                None => schema["additionalProperties"] != json!(false),
            })
}

fn items_valid(schema: &Value, value: &Value) -> bool {
    match (value.as_array(), schema.get("items")) {
        (Some(items), Some(sub)) => items.iter().all(|v| valid(sub, v)),
        _ => true,
    }
}

fn bounds_valid(schema: &Value, value: &Value) -> bool {
    match (schema["minimum"].as_f64(), value.as_f64()) {
        (Some(min), Some(n)) => n >= min,
        _ => true,
    }
}

fn valid(schema: &Value, value: &Value) -> bool {
    if let Some(options) = schema["anyOf"].as_array() {
        return options.iter().any(|o| valid(o, value));
    }

    let allowed = schema["enum"].as_array().is_none_or(|e| e.contains(value));
    allowed
        && type_valid(schema, value)
        && bounds_valid(schema, value)
        && members_valid(schema, value)
        && items_valid(schema, value)
}

fn rule_schema(schema: &Value, group: &str, rule: &str) -> Value {
    let rules = &schema["properties"]["rules"];
    let options = rules["anyOf"].as_array().expect("rules anyOf");
    options
        .iter()
        .find_map(|o| o["properties"][group]["properties"].get(rule))
        .unwrap_or_else(|| panic!("schema has no rules.{group}.{rule}: {rules}"))
        .clone()
}

fn accepted(rule: &str, value: Value) -> bool {
    valid(&rule_schema(&committed(), "core", rule), &value)
}

#[test]
fn object_and_list_forms_validate_for_both_rules() {
    for rule in ["cognitive-complexity", "function-length"] {
        for value in [
            json!("error"),
            json!("warn"),
            json!("off"),
            json!({"warn": 10}),
            json!({"error": 20}),
            json!({"warn": 10, "error": 20, "include": ["scripts/**"]}),
            json!([{"error": 15}, {"warn": 10, "error": 20, "include": ["scripts/**"]}]),
            json!([]),
        ] {
            assert!(accepted(rule, value.clone()), "{rule}: {value}");
        }
    }

    assert!(accepted(
        "function-length",
        json!({"warn": 60, "skipBlankLines": true})
    ));
}

#[test]
fn wrong_types_and_old_keys_are_rejected() {
    for rule in ["cognitive-complexity", "function-length"] {
        for value in [
            json!({"warn": "x"}),
            json!({"error": 1.5}),
            json!({"error": 0}),
            json!([{"warn": "x"}]),
            json!([{"error": 5, "bogus": 1}]),
            json!("loud"),
        ] {
            assert!(!accepted(rule, value.clone()), "{rule}: {value}");
        }
    }

    assert!(!accepted(
        "cognitive-complexity",
        json!({"warn": 5, "skipBlankLines": true})
    ));
}

#[test]
fn generated_schema_documents_warn_error_and_the_list_form() {
    let generated = klint_rs::config_schema("0.0.0");

    for rule in ["cognitive-complexity", "function-length"] {
        let sub = rule_schema(&generated, "core", rule);
        let text = sub.to_string();
        assert!(
            text.contains("\"warn\"") && text.contains("\"error\""),
            "{rule}"
        );
        assert!(text.contains("\"items\""), "{rule}: list form missing");
        assert!(valid(
            &sub,
            &json!([{"error": 15}, {"warn": 10, "include": ["a/**"]}])
        ));
        assert!(!valid(&sub, &json!({"warn": "x"})));
    }
}

#[test]
fn committed_schema_equals_the_generated_one_for_the_threshold_rules() {
    let generated = klint_rs::config_schema("0.0.0");

    for rule in ["cognitive-complexity", "function-length"] {
        assert_eq!(
            rule_schema(&committed(), "core", rule),
            rule_schema(&generated, "core", rule),
            "regenerate klint.schema.json"
        );
    }
}
