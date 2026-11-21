mod common;

use common::temp_root;
use serde_json::Value;
use std::fs::{create_dir_all, read_to_string, remove_dir_all, write};
use std::path::{Path, PathBuf};
use std::process::Command;

const SOURCE: &str = "console.log(\"x\");\nif (a) {}\n";
const FIXED: &str = "console.log(\"x\");\n\nif (a) {}\n";

struct Outcome {
    exit: i32,
    stdout: String,
    stderr: String,
}

struct Project(PathBuf);

impl Drop for Project {
    fn drop(&mut self) {
        let _ = remove_dir_all(&self.0);
    }
}

fn project(name: &str) -> Project {
    let root = temp_root(name);
    create_dir_all(root.join("src")).expect("create fixture dirs");
    write(
        root.join("klint.yaml"),
        r#"
include: ["src"]
rules:
  core:
    blank-line-before-control-flow: error
arch:
  forbidden:
    - pattern: "console.log("
      in: "src/**"
      message: "Use logger"
"#,
    )
    .expect("write config");
    write(root.join("src/index.ts"), SOURCE).expect("write source");
    Project(root)
}

fn klint(root: &Path, args: &[&str]) -> Outcome {
    let output = Command::new(env!("CARGO_BIN_EXE_klint-rs"))
        .args(["--config", &root.to_string_lossy()])
        .args(args)
        .env("KLINT_NO_STATS", "1")
        .output()
        .expect("klint-rs runs");
    Outcome {
        exit: output.status.code().expect("exits normally"),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn source_of(root: &Path) -> String {
    read_to_string(root.join("src/index.ts")).expect("read source")
}

#[test]
fn fix_with_the_json_reporter_prints_what_it_applied_and_what_remains() {
    let project = project("fix-json");
    let root = &project.0;
    let out = klint(root, &["--fix", "--reporter=json"]);
    let json: Value = serde_json::from_str(&out.stdout)
        .unwrap_or_else(|err| panic!("{err}: {} {}", out.stdout, out.stderr));

    assert_eq!(out.exit, 0, "{}", out.stderr);
    assert_eq!(out.stderr, "");
    assert_eq!(json["applied"], 1);
    assert_eq!(json["summary"]["errors"], 1);
    assert_eq!(json["violations"].as_array().map(Vec::len), Some(1));
    assert_eq!(json["violations"][0]["rule"], "arch/forbidden");
    assert_eq!(source_of(root), FIXED);
}

#[test]
fn fix_with_the_json_alias_prints_the_same_json() {
    let project = project("fix-json-alias");
    let root = &project.0;
    let named = klint(root, &["--fix", "--reporter", "json"]);
    write(root.join("src/index.ts"), SOURCE).expect("reset source");
    let alias = klint(root, &["--fix", "--json"]);

    assert_eq!(alias.exit, named.exit);
    assert_eq!(alias.stdout, named.stdout);
}

#[test]
fn fix_with_the_github_reporter_is_refused_and_changes_nothing() {
    let project = project("fix-github");
    let root = &project.0;
    let out = klint(root, &["--fix", "--reporter=github"]);

    assert_eq!(out.exit, 1);
    assert_eq!(out.stdout, "");
    assert_eq!(
        out.stderr,
        "klint: --fix does not support --reporter github; use --reporter text or json\n"
    );
    assert_eq!(source_of(root), SOURCE);
}

#[test]
fn fix_with_the_text_reporter_keeps_the_text_summary() {
    let project = project("fix-text");
    let root = &project.0;
    let default = klint(root, &["--fix"]);
    write(root.join("src/index.ts"), SOURCE).expect("reset source");
    let named = klint(root, &["--fix", "--reporter=text"]);

    assert_eq!(default.exit, 0);
    assert!(
        default
            .stdout
            .starts_with("klint: applied 1 fix(es). 1 violation(s) require manual attention."),
        "{}",
        default.stdout
    );
    assert_eq!(
        strip_duration(&named.stdout),
        strip_duration(&default.stdout)
    );
}

fn strip_duration(text: &str) -> &str {
    text.split(" Finished in").next().unwrap_or(text)
}
