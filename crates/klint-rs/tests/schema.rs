use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn repo_file(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn committed_version(schema: &Value) -> String {
    let id = schema["$id"].as_str().expect("committed schema has an $id");
    id.split("/refs/tags/v")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .expect("$id names a tagged version")
        .to_string()
}

#[test]
fn committed_json_schema_matches_the_rust_config_types() {
    let committed: Value =
        serde_json::from_str(&repo_file("klint.schema.json")).expect("klint.schema.json is JSON");

    assert_eq!(
        committed,
        klint_rs::config_schema(&committed_version(&committed)),
        "klint.schema.json is stale — run `bun run schema`"
    );
}

#[test]
fn committed_yaml_schema_matches_the_json_schema() {
    let json: Value =
        serde_json::from_str(&repo_file("klint.schema.json")).expect("klint.schema.json is JSON");
    let yaml: Value =
        serde_yaml::from_str(&repo_file("klint.schema.yaml")).expect("klint.schema.yaml is YAML");

    assert_eq!(
        yaml, json,
        "klint.schema.yaml is stale — run `bun run schema`"
    );
}

fn generated_schema() -> Value {
    klint_rs::config_schema("1.5.0")
}

fn v2_group(schema: &Value, group: &str) -> Value {
    let branches = schema["properties"]["rules"]["anyOf"]
        .as_array()
        .expect("rules has an anyOf");
    let v2 = branches
        .iter()
        .find(|branch| branch["properties"]["core"].is_object())
        .expect("a v2 branch");
    v2["properties"][group]["properties"].clone()
}

fn v1_map(schema: &Value) -> Value {
    let branches = schema["properties"]["rules"]["anyOf"]
        .as_array()
        .expect("rules has an anyOf");
    branches
        .iter()
        .find(|branch| branch["properties"]["core"].is_null())
        .expect("a v1 branch")["properties"]
        .clone()
}

fn assert_experimental_group_exists(schema: &Value) {
    assert!(
        v2_group(schema, "experimental").is_object(),
        "rules.experimental is missing from the generated schema"
    );
}

#[test]
fn no_string_match_is_listed_under_core_and_not_under_experimental() {
    let schema = generated_schema();
    assert_experimental_group_exists(&schema);
    assert!(v2_group(&schema, "core").get("no-string-match").is_some());
    assert!(
        v2_group(&schema, "experimental")
            .get("no-string-match")
            .is_none()
    );
}

#[test]
fn no_date_equality_is_listed_only_under_experimental() {
    let schema = generated_schema();
    assert_experimental_group_exists(&schema);
    assert!(
        v2_group(&schema, "experimental")
            .get("no-date-equality")
            .is_some()
    );
    assert!(v2_group(&schema, "core").get("no-date-equality").is_none());
    assert!(v1_map(&schema).get("no-date-equality").is_none());
}

#[test]
fn no_floating_promise_is_listed_only_under_experimental() {
    let schema = generated_schema();
    assert_experimental_group_exists(&schema);
    assert!(
        v2_group(&schema, "experimental")
            .get("no-floating-promise")
            .is_some()
    );
    assert!(
        v2_group(&schema, "core")
            .get("no-floating-promise")
            .is_none()
    );
    assert!(v1_map(&schema).get("no-floating-promise").is_none());
}

#[test]
fn sonar_prefixed_name_is_listed_under_neither_core_nor_experimental() {
    let schema = generated_schema();
    assert_experimental_group_exists(&schema);
    assert!(
        v2_group(&schema, "experimental")
            .get("sonar/prefer-at")
            .is_none()
    );
    assert!(v2_group(&schema, "core").get("sonar/prefer-at").is_none());
}

fn arch_examples(schema: &Value) -> Vec<(String, String)> {
    let sections = schema["properties"]["arch"]["properties"]
        .as_object()
        .expect("arch has properties");
    sections
        .iter()
        .flat_map(|(name, node)| {
            [&node["examples"], &node["items"]["examples"]]
                .into_iter()
                .filter_map(Value::as_array)
                .flatten()
                .filter_map(Value::as_str)
                .map(|example| (name.clone(), example.to_string()))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn is_glob(scope: &str) -> bool {
    scope.contains(['/', '*', '.'])
}

fn scope_names(entry: &serde_yaml::Value) -> Vec<String> {
    ["in", "from", "deny", "allow"]
        .iter()
        .filter_map(|key| entry.get(*key))
        .flat_map(|value| match value {
            serde_yaml::Value::Sequence(items) => items.clone(),
            other => vec![other.clone()],
        })
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect()
}

#[test]
fn every_arch_example_defines_the_layers_it_names() {
    for (section, example) in arch_examples(&generated_schema()) {
        let doc: serde_yaml::Value = serde_yaml::from_str(&example).expect("example is YAML");
        let arch = &doc["arch"];
        let defined: Vec<String> = arch["layers"]
            .as_mapping()
            .map(|layers| {
                layers
                    .keys()
                    .filter_map(|k| k.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let entries = arch[section.as_str()]
            .as_sequence()
            .cloned()
            .unwrap_or_default();

        for name in entries.iter().flat_map(scope_names) {
            assert!(
                is_glob(&name) || defined.contains(&name),
                "arch.{section} example names layer `{name}` without defining it in `layers`:\n{example}"
            );
        }
    }
}
