use super::*;
use std::path::Path;

fn entries(file: &str, content: &str) -> Vec<(String, Option<String>, String)> {
    config_entries(Path::new(file), content)
        .expect("parses")
        .into_iter()
        .map(|entry| {
            (
                entry.path.join("."),
                entry.value,
                content[entry.key].to_string(),
            )
        })
        .collect()
}

fn entry(path: &str, value: Option<&str>, key: &str) -> (String, Option<String>, String) {
    (path.to_string(), value.map(str::to_string), key.to_string())
}

#[test]
fn json_with_comments_gives_paths_values_and_key_text() {
    let found = entries(
        "a.jsonc",
        "{\n  // c\n  \"a\": {\"b\": [1, \"x\"]},\n  \"t\": true\n}\n",
    );

    assert_eq!(
        found,
        vec![
            entry("a", None, "\"a\""),
            entry("a.b", None, "\"b\""),
            entry("a.b.0", Some("1"), "1"),
            entry("a.b.1", Some("x"), "\"x\""),
            entry("t", Some("true"), "\"t\""),
        ]
    );
}

#[test]
fn yaml_block_and_flow_forms_give_the_same_paths() {
    let found = entries("a.yaml", "a:\n  b: [1, x]\n  'q': \"s\"\nl:\n  - k: v\n");

    assert_eq!(
        found,
        vec![
            entry("a", None, "a"),
            entry("a.b", None, "b"),
            entry("a.b.0", Some("1"), "1"),
            entry("a.b.1", Some("x"), "x"),
            entry("a.q", Some("s"), "'q'"),
            entry("l", None, "l"),
            entry("l.0", None, "-"),
            entry("l.0.k", Some("v"), "k"),
        ]
    );
}

#[test]
fn toml_tables_dotted_keys_and_array_tables_give_paths() {
    let found = entries(
        "Cargo.toml",
        "[package]\nedition = \"2021\"\n[dependencies.serde]\nversion = \"1\"\n[[bin]]\nname = \"x\"\n[[bin]]\nname = \"y\"\n",
    );

    assert_eq!(
        found,
        vec![
            entry("package", None, "package"),
            entry("package.edition", Some("2021"), "edition"),
            entry("dependencies.serde", None, "dependencies.serde"),
            entry("dependencies.serde.version", Some("1"), "version"),
            entry("bin.0", None, "bin"),
            entry("bin.0.name", Some("x"), "name"),
            entry("bin.1", None, "bin"),
            entry("bin.1.name", Some("y"), "name"),
        ]
    );
}

#[test]
fn a_file_with_a_syntax_error_gives_no_entries() {
    assert_eq!(config_entries(Path::new("a.json"), "{\"a\": }\n"), None);
}

#[test]
fn star_matches_one_segment_and_parents_are_found_by_prefix() {
    let pattern: Vec<String> = ["dependencies", "*"].map(String::from).to_vec();
    let path: Vec<String> = ["dependencies", "serde"].map(String::from).to_vec();
    let deeper: Vec<String> = ["dependencies", "serde", "version"]
        .map(String::from)
        .to_vec();

    assert!(path_matches(&pattern, &path));
    assert!(!path_matches(&pattern, &deeper));
    assert!(path_starts_with(&deeper, &pattern));
}
