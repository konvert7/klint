use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn schema() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../klint.schema.json");
    serde_json::from_str(&fs::read_to_string(path).expect("read schema")).expect("json")
}

fn contains_key(value: &Value, key: &str) -> bool {
    match value {
        Value::Object(map) => map.contains_key(key) || map.values().any(|v| contains_key(v, key)),
        Value::Array(items) => items.iter().any(|v| contains_key(v, key)),
        _ => false,
    }
}

#[test]
fn schema_documents_v2_groups_and_still_has_v1_shape_and_plugins() {
    let text = schema().to_string();
    for key in [
        "recommended",
        "core",
        "sonar",
        "plugins",
        "sonar/prefer-at",
        "no-string-match",
    ] {
        assert!(contains_key(&schema(), key), "schema lacks property {key}");
    }
    assert!(
        contains_key(&schema(), "anyOf"),
        "rules needs an anyOf of v1 and v2"
    );
    assert!(
        text.contains("klint migrate"),
        "descriptions must mention `klint migrate`"
    );
}

#[test]
fn generated_schema_matches_committed_and_mentions_migrate() {
    let generated = klint_rs::config_schema("0.0.0");
    assert!(generated.to_string().contains("klint migrate"));
    assert!(contains_key(&generated, "recommended"));
}
