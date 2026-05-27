use serde_json::Value;
use std::fs::{create_dir_all, remove_dir_all, write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_root(name: &str) -> PathBuf {
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for tests")
        .as_nanos();
    std::env::temp_dir().join(format!("klint-rs-cli-{name}-{id}"))
}

fn run_cli(root: &Path) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_klint-rs"))
        .args(["--config", &root.to_string_lossy(), "--json"])
        .output()
        .expect("klint-rs binary should execute");

    (
        output.status.code().expect("process should exit normally"),
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        String::from_utf8(output.stderr).expect("stderr should be utf8"),
    )
}

fn parse_json(stdout: &str) -> Value {
    serde_json::from_str(stdout).expect("stdout should be JSON")
}

#[test]
fn cli_version_exits_zero() {
    let output = Command::new(env!("CARGO_BIN_EXE_klint-rs"))
        .arg("--version")
        .output()
        .expect("klint-rs binary should execute");

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        format!("klint-rs {}\n", env!("CARGO_PKG_VERSION"))
    );
    assert!(
        String::from_utf8(output.stderr)
            .expect("stderr should be utf8")
            .is_empty()
    );
}

#[test]
fn cli_help_mentions_version_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_klint-rs"))
        .arg("--help")
        .output()
        .expect("klint-rs binary should execute");

    assert_eq!(output.status.code(), Some(0));
    assert!(
        String::from_utf8(output.stdout)
            .expect("stdout should be utf8")
            .contains("--version")
    );
    assert!(
        String::from_utf8(output.stderr)
            .expect("stderr should be utf8")
            .is_empty()
    );
}

#[test]
fn cli_json_clean_run_exits_zero() {
    let root = temp_root("clean");
    create_dir_all(root.join("src")).expect("create fixture dirs");
    write(root.join("klint.yaml"), "include: [\"src\"]\nrules: {}\n").expect("write config");
    write(root.join("src/index.ts"), "export const value = 1;\n").expect("write source");

    let (status, stdout, stderr) = run_cli(&root);
    let json = parse_json(&stdout);

    assert_eq!(status, 0, "{stderr}");
    assert_eq!(json["violations"], Value::Array(Vec::new()));
    assert_eq!(json["summary"]["errors"], 0);
    assert_eq!(json["summary"]["warnings"], 0);
    assert!(stderr.is_empty());
    let _ = remove_dir_all(root);
}

#[test]
fn cli_json_error_violation_exits_two() {
    let root = temp_root("error");
    create_dir_all(root.join("src")).expect("create fixture dirs");
    write(
        root.join("klint.yaml"),
        r#"
include: ["src"]
rules: {}
arch:
  forbidden:
    - pattern: "console.log("
      in: "src/**"
      message: "Use logger"
"#,
    )
    .expect("write config");
    write(root.join("src/index.ts"), "console.log(\"x\");\n").expect("write source");

    let (status, stdout, stderr) = run_cli(&root);
    let json = parse_json(&stdout);

    assert_eq!(status, 2, "{stderr}");
    assert_eq!(json["summary"]["errors"], 1);
    assert_eq!(json["summary"]["warnings"], 0);
    assert_eq!(json["violations"][0]["file"], "src/index.ts");
    assert_eq!(json["violations"][0]["line"], 1);
    assert_eq!(json["violations"][0]["rule"], "arch/forbidden");
    assert_eq!(json["violations"][0]["message"], "Use logger");
    assert_eq!(json["violations"][0]["severity"], "error");
    assert_eq!(json["violations"][0]["fix"], Value::Null);
    assert!(stderr.is_empty());
    let _ = remove_dir_all(root);
}

#[test]
fn cli_json_warning_only_violation_exits_zero() {
    let root = temp_root("warning");
    create_dir_all(root.join("src")).expect("create fixture dirs");
    write(
        root.join("klint.yaml"),
        r#"
include: ["src"]
rules: {}
arch:
  forbidden:
    - pattern: "console.log("
      in: "src/**"
      message: "Use logger"
      severity: warn
"#,
    )
    .expect("write config");
    write(root.join("src/index.ts"), "console.log(\"x\");\n").expect("write source");

    let (status, stdout, stderr) = run_cli(&root);
    let json = parse_json(&stdout);

    assert_eq!(status, 0, "{stderr}");
    assert_eq!(json["summary"]["errors"], 0);
    assert_eq!(json["summary"]["warnings"], 1);
    assert_eq!(json["violations"][0]["severity"], "warn");
    assert!(stderr.is_empty());
    let _ = remove_dir_all(root);
}
