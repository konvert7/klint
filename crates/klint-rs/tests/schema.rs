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
