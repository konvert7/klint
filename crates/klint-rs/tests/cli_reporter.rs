mod common;

use common::temp_root;
use std::fs::{create_dir_all, remove_dir_all, write};
use std::path::{Path, PathBuf};
use std::process::Command;

struct Outcome {
    exit: i32,
    stdout: String,
    stderr: String,
}

fn project_with_one_error(name: &str) -> PathBuf {
    let root = temp_root(name);
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
    root
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

#[test]
fn json_flag_and_json_reporter_print_the_same_output() {
    let root = project_with_one_error("reporter-json-alias");
    let alias = klint(&root, &["--json"]);

    for args in [&["--reporter=json"][..], &["--reporter", "json"][..]] {
        let named = klint(&root, args);
        assert_eq!(named.exit, alias.exit, "{args:?}");
        assert_eq!(named.stdout, alias.stdout, "{args:?}");
        assert_eq!(named.stderr, alias.stderr, "{args:?}");
    }

    assert_eq!(alias.exit, 2);
    assert!(alias.stdout.contains("\"rule\":\"arch/forbidden\""));
    let _ = remove_dir_all(root);
}

#[test]
fn github_reporter_prints_one_annotation_per_violation() {
    let root = project_with_one_error("reporter-github");
    let out = klint(&root, &["--reporter=github"]);

    assert_eq!(out.exit, 2, "{}", out.stderr);
    assert_eq!(
        out.stdout,
        "::error file=src/index.ts,line=1,title=arch/forbidden::Use logger\n"
    );
    assert_eq!(out.stderr, "");
    let _ = remove_dir_all(root);
}

#[test]
fn text_reporter_is_the_default() {
    let root = project_with_one_error("reporter-text");
    let default = klint(&root, &[]);
    let named = klint(&root, &["--reporter=text"]);

    assert_eq!(named.exit, default.exit);
    assert_eq!(named.stdout, default.stdout);
    assert!(named.stderr.contains("Use logger"));
    let _ = remove_dir_all(root);
}

#[test]
fn unknown_reporter_exits_one_and_lists_the_valid_names() {
    let root = project_with_one_error("reporter-unknown");
    let out = klint(&root, &["--reporter=sarif"]);

    assert_eq!((out.exit, out.stdout.as_str()), (1, ""));
    assert_eq!(
        out.stderr,
        "klint: unknown reporter 'sarif', expected one of: text, json, github\n"
    );
    let _ = remove_dir_all(root);
}

#[test]
fn json_flag_with_another_reporter_is_rejected() {
    let root = project_with_one_error("reporter-conflict");
    let conflict = klint(&root, &["--json", "--reporter=github"]);
    let agreeing = klint(&root, &["--json", "--reporter=json"]);

    assert_eq!((conflict.exit, conflict.stdout.as_str()), (1, ""));
    assert!(conflict.stderr.contains("pass only one reporter"));
    assert_eq!(agreeing.exit, 2);
    let _ = remove_dir_all(root);
}

#[test]
fn help_lists_reporter_and_hides_the_json_alias() {
    let root = project_with_one_error("reporter-help");
    let out = klint(&root, &["--help"]);

    assert!(out.stdout.contains("--reporter <name>"), "{}", out.stdout);
    assert!(out.stdout.contains("text, json, github"), "{}", out.stdout);
    assert!(!out.stdout.contains("--json"), "{}", out.stdout);
    let _ = remove_dir_all(root);
}
