mod common;

use common::temp_root;
use klint_rs::{RunOptions, fix, run};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::fs::{create_dir_all, read_to_string, remove_dir_all, write};
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct ParityCase {
    name: String,
    #[serde(default)]
    config: Map<String, Value>,
    #[serde(rename = "configFile")]
    config_file: Option<String>,
    #[serde(rename = "configText")]
    config_text: Option<String>,
    files: BTreeMap<String, String>,
    expected: Vec<Value>,
    #[serde(rename = "fixedFiles")]
    fixed_files: Option<BTreeMap<String, String>>,
    #[serde(rename = "textOutput")]
    text_output: Option<Value>,
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

    if let Some(text) = &case.config_text {
        let file = case.config_file.as_deref().unwrap_or("klint.yaml");
        write(root.join(file), with_current_version(text)).expect("write case config text");

        return;
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
    let Ok(output) = run(RunOptions {
        config_dir: root.to_path_buf(),
    }) else {
        return Vec::new();
    };
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

fn files_after_rust_fix(root: &Path, case: &ParityCase) -> BTreeMap<String, String> {
    let _ = fix(RunOptions {
        config_dir: root.to_path_buf(),
    });
    case.files
        .iter()
        .filter_map(|(relative, before)| {
            let after = read_to_string(root.join(relative)).expect("read fixed file");
            (after != *before).then(|| (relative.clone(), after))
        })
        .collect()
}

#[test]
fn rust_fix_leaves_the_same_files_as_the_ts_engine_fix() {
    let cases = corpus();
    let fixable: Vec<&ParityCase> = cases
        .iter()
        .filter(|case| case.fixed_files.is_some())
        .collect();
    assert!(fixable.len() >= 48, "only {} fixable cases", fixable.len());

    let mismatches: Vec<String> = fixable
        .iter()
        .filter_map(|case| {
            let root = temp_root("parity-fix");
            write_case(&root, case);
            let actual = files_after_rust_fix(&root, case);
            let _ = remove_dir_all(&root);
            let expected = case.fixed_files.clone().unwrap_or_default();
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
        "{} of {} fix cases differ:\n{}",
        mismatches.len(),
        fixable.len(),
        mismatches.join("\n")
    );
}

const VERSION_PLACEHOLDER: &str = "<version>";

fn with_current_version(text: &str) -> String {
    text.replace(VERSION_PLACEHOLDER, env!("CARGO_PKG_VERSION"))
}

fn comparable_text(text: &str, root: &Path) -> String {
    let duration = regex::Regex::new(r"in \d+(\.\d+)?m?s\b").expect("valid duration regex");
    duration
        .replace_all(text, "in <t>")
        .replace(root.to_string_lossy().as_ref(), "<dir>")
        .replace("<dir>\\", "<dir>/")
        .replace(env!("CARGO_PKG_VERSION"), VERSION_PLACEHOLDER)
}

#[test]
fn rust_cli_prints_and_exits_like_the_ts_engine() {
    let cases = corpus();
    let recorded: Vec<&ParityCase> = cases
        .iter()
        .filter(|case| case.text_output.is_some())
        .collect();
    assert!(recorded.len() >= 203, "only {} text cases", recorded.len());

    let mismatches: Vec<String> = recorded
        .iter()
        .filter_map(|case| {
            let root = temp_root("parity-text");
            write_case(&root, case);
            let output = std::process::Command::new(env!("CARGO_BIN_EXE_klint-rs"))
                .args(["--config", &root.to_string_lossy()])
                .env("KLINT_NO_STATS", "1")
                .output()
                .expect("klint-rs runs");
            let actual = json!({
                "exit": output.status.code(),
                "stdout": comparable_text(&String::from_utf8_lossy(&output.stdout), &root),
                "stderr": comparable_text(&String::from_utf8_lossy(&output.stderr), &root),
            });
            let _ = remove_dir_all(&root);
            let expected = case.text_output.clone().unwrap_or_default();
            (actual != expected).then(|| {
                format!(
                    "{}\n  expected: {expected}\n  actual:   {actual}",
                    case.name
                )
            })
        })
        .collect();

    assert!(
        mismatches.is_empty(),
        "{} of {} text cases differ:\n{}",
        mismatches.len(),
        recorded.len(),
        mismatches.join("\n")
    );
}

#[derive(Deserialize)]
struct StatsStep {
    files: BTreeMap<String, String>,
    #[serde(default)]
    hook: bool,
}

#[derive(Deserialize)]
struct StatsSequence {
    name: String,
    #[serde(rename = "configText")]
    config_text: String,
    #[serde(rename = "initialStats")]
    initial_stats: Option<String>,
    steps: Vec<StatsStep>,
    #[serde(rename = "statsAfter")]
    stats_after: Vec<Value>,
    gitignore: String,
    #[serde(rename = "statsText")]
    stats_text: Value,
}

fn stats_corpus() -> Vec<StatsSequence> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/parity/stats.jsonl");
    read_to_string(&path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
        .lines()
        .map(|line| serde_json::from_str(line).expect("each stats line is a sequence"))
        .collect()
}

fn write_tree(root: &Path, files: &BTreeMap<String, String>) {
    for (relative, content) in files {
        let file = root.join(relative);
        create_dir_all(file.parent().expect("file has a parent")).expect("create dir");
        write(file, content).expect("write file");
    }
}

fn klint_rs(root: &Path, args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_klint-rs"))
        .args(args)
        .args(["--config", &root.to_string_lossy()])
        .env_remove("KLINT_NO_STATS")
        .output()
        .expect("klint-rs runs")
}

fn stats_mismatches(sequence: &StatsSequence) -> Vec<String> {
    let root = temp_root("parity-stats");
    create_dir_all(&root).expect("create root");
    write(root.join("klint.yaml"), &sequence.config_text).expect("write config");

    if let Some(initial) = &sequence.initial_stats {
        create_dir_all(root.join(".klint")).expect("create .klint");
        write(root.join(".klint/stats.json"), initial).expect("write initial stats");
    }

    let mut mismatches = Vec::new();

    for (index, (step, expected)) in sequence.steps.iter().zip(&sequence.stats_after).enumerate() {
        write_tree(&root, &step.files);
        klint_rs(&root, if step.hook { &["--hook"] } else { &[] });
        let actual: Value = serde_json::from_str(
            &read_to_string(root.join(".klint/stats.json")).expect("stats written"),
        )
        .expect("stats is JSON");
        if actual != *expected {
            mismatches.push(format!(
                "step {index}\n  expected: {expected}\n  actual:   {actual}"
            ));
        }
    }

    let gitignore = read_to_string(root.join(".klint/.gitignore")).unwrap_or_default();
    if gitignore != sequence.gitignore {
        mismatches.push(format!("gitignore {gitignore:?}"));
    }

    let output = klint_rs(&root, &["stats"]);
    let text = json!({
        "exit": output.status.code(),
        "stdout": String::from_utf8_lossy(&output.stdout),
        "stderr": String::from_utf8_lossy(&output.stderr),
    });
    if text != sequence.stats_text {
        mismatches.push(format!(
            "stats text\n  expected: {}\n  actual:   {text}",
            sequence.stats_text
        ));
    }

    let _ = remove_dir_all(&root);
    mismatches
}

#[test]
fn rust_stats_continue_the_ts_engine_stats_file() {
    let sequences = stats_corpus();
    assert!(
        sequences.len() >= 4,
        "only {} stats sequences",
        sequences.len()
    );

    let mismatches: Vec<String> = sequences
        .iter()
        .flat_map(|sequence| {
            stats_mismatches(sequence)
                .into_iter()
                .map(move |mismatch| format!("{}: {mismatch}", sequence.name))
        })
        .collect();

    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}
