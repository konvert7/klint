mod common;

use common::temp_root;
use serde_json::Value;
use std::fs::{create_dir_all, remove_dir_all, write};
use std::path::Path;
use std::process::Command;

const CONFIG: &str = "include: [\"src\"]\nrules:\n  no-async-predicate: error\n";

const CASES_THE_TS_ENGINE_ALSO_DECIDES: &str = r#"declare const arr: number[], ro: readonly string[], maybe: number[] | undefined;
declare const notArray: { filter(f: (x: number) => unknown): void };
async function isOk(n: number) { return n > 0; }
arr.filter(async (n) => n > 1);
ro.some(async function (s) { return s.length > 0; });
arr.find(async(n)=>n === 1);
maybe?.every(async (n) => n > 0);
arr
  .findIndex(
    async (n) => n > 2
  );
arr.filter((n) => n > 1);
arr.map(async (n) => n);
arr.filter(isOk);
"#;

fn lines_flagged(source: &str) -> (i32, Vec<u64>, Vec<String>) {
    let root = temp_root("async-predicate");
    create_dir_all(root.join("src")).expect("create fixture dirs");
    write(root.join("klint.yaml"), CONFIG).expect("write config");
    write(root.join("src/index.ts"), source).expect("write source");
    let (status, json) = run_json(&root);
    let _ = remove_dir_all(&root);
    let violations = json["violations"].as_array().cloned().unwrap_or_default();
    (
        status,
        violations
            .iter()
            .map(|v| v["line"].as_u64().expect("line"))
            .collect(),
        violations
            .iter()
            .map(|v| v["message"].as_str().expect("message").to_string())
            .collect(),
    )
}

fn run_json(root: &Path) -> (i32, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_klint-rs"))
        .args(["--config", &root.to_string_lossy(), "--json"])
        .env("KLINT_NO_STATS", "1")
        .output()
        .expect("klint-rs binary should execute");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    (
        output.status.code().expect("process should exit normally"),
        serde_json::from_str(&stdout).unwrap_or(Value::Null),
    )
}

#[test]
fn flags_the_same_lines_as_the_ts_engine() {
    let (status, lines, messages) = lines_flagged(CASES_THE_TS_ENGINE_ALSO_DECIDES);

    assert_eq!(status, 2);
    assert_eq!(lines, vec![4, 5, 6, 7, 10, 1]);
    assert_eq!(
        messages[0],
        "Async callback passed to .filter() — the returned Promise is always truthy, so the predicate never filters correctly. The array method cannot await it."
    );
    assert!(messages[4].starts_with("Async callback passed to .findIndex()"));
}

#[test]
fn flags_any_receiver_because_it_cannot_tell_arrays_apart_without_types() {
    let (_, lines, _) = lines_flagged(
        "declare const notArray: { filter(f: (x: number) => unknown): void };\nnotArray.filter(async (n) => n > 0);\n",
    );

    assert_eq!(lines, vec![2, 1]);
}
