mod common;

use common::temp_root;
use klint_rs::{RunOptions, run};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::fs::{create_dir_all, read_to_string, remove_dir_all, write};
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct ParityCase {
    name: String,
    config: Map<String, Value>,
    files: BTreeMap<String, String>,
    expected: Vec<Value>,
}

fn corpus() -> Vec<ParityCase> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/parity/cases.jsonl");
    read_to_string(&path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
        .lines()
        .map(|line| serde_json::from_str(line).expect("each corpus line is a case"))
        .collect()
}

fn write_case(root: &Path, case: &ParityCase) {
    for (relative, content) in &case.files {
        let file = root.join(relative);
        create_dir_all(file.parent().expect("case file has a parent")).expect("create case dir");
        write(file, content).expect("write case file");
    }
    let mut config = case.config.clone();
    config.insert("root".to_string(), json!("."));
    write(
        root.join("klint.config.json"),
        serde_json::to_string(&config).expect("config serializes"),
    )
    .expect("write case config");
}

fn sorted_findings(findings: Vec<Value>) -> Vec<String> {
    let mut keys: Vec<String> = findings
        .into_iter()
        .map(|finding| serde_json::to_string(&finding).expect("finding serializes"))
        .collect();
    keys.sort();
    keys
}

fn rust_findings(root: &Path) -> Vec<Value> {
    let output = run(RunOptions {
        config_dir: root.to_path_buf(),
    })
    .expect("parity case config should run");
    output
        .violations
        .into_iter()
        .filter(|violation| !violation.rule.starts_with("klint/"))
        .map(|violation| {
            json!({
                "file": violation.file,
                "line": violation.line,
                "rule": violation.rule,
                "severity": violation.severity,
                "message": violation.message,
                "fix": violation.fix,
            })
        })
        .collect()
}

#[test]
fn rust_engine_matches_every_case_recorded_from_the_ts_engine_tests() {
    let cases = corpus();
    assert!(cases.len() >= 221, "corpus shrank to {} cases", cases.len());

    let mismatches: Vec<String> = cases
        .iter()
        .filter_map(|case| {
            let root = temp_root("parity");
            write_case(&root, case);
            let actual = sorted_findings(rust_findings(&root));
            let _ = remove_dir_all(&root);
            let expected = sorted_findings(case.expected.clone());
            (actual != expected).then(|| {
                format!(
                    "{}\n  expected: {expected:?}\n  actual:   {actual:?}",
                    case.name
                )
            })
        })
        .collect();

    assert!(
        mismatches.is_empty(),
        "{} of {} parity cases differ:\n{}",
        mismatches.len(),
        cases.len(),
        mismatches.join("\n")
    );
}
