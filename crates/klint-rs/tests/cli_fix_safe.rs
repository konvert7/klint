mod common;

use common::temp_root;
use std::fs::{create_dir_all, read_to_string, remove_dir_all, write};
use std::path::{Path, PathBuf};
use std::process::Command;

const SOURCE: &str = "foo();\nif (a) {}\nconst m = s.match(/x/);\n";
const LAYOUT_FIXED: &str = "foo();\n\nif (a) {}\nconst m = s.match(/x/);\n";
const ALL_FIXED: &str = "foo();\n\nif (a) {}\nconst m = /x/.exec(s);\n";

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
        "include: [\"src\"]\nrules:\n  core:\n    blank-line-before-control-flow: error\n    no-string-match: error\n",
    )
    .expect("write config");
    write(root.join("src/a.ts"), SOURCE).expect("write a");
    write(root.join("src/b.ts"), SOURCE).expect("write b");
    Project(root)
}

fn klint(root: &Path, args: &[&str]) -> (i32, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_klint-rs"))
        .current_dir(root)
        .args(args)
        .env("KLINT_NO_STATS", "1")
        .output()
        .expect("klint-rs runs");
    (
        output.status.code().expect("exits normally"),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn source(root: &Path, file: &str) -> String {
    read_to_string(root.join("src").join(file)).expect("read source")
}

#[test]
fn fix_safe_applies_layout_fixes_and_leaves_code_rewrites() {
    let project = project("fix-safe");
    let (exit, stderr) = klint(&project.0, &["--fix=safe"]);

    assert_eq!(exit, 0, "{stderr}");
    assert_eq!(source(&project.0, "a.ts"), LAYOUT_FIXED);
    assert_eq!(source(&project.0, "b.ts"), LAYOUT_FIXED);
}

#[test]
fn plain_fix_still_applies_every_fix() {
    let project = project("fix-all");
    let (exit, stderr) = klint(&project.0, &["--fix"]);

    assert_eq!(exit, 0, "{stderr}");
    assert_eq!(source(&project.0, "a.ts"), ALL_FIXED);
}

#[test]
fn paths_limit_the_fix_to_those_files() {
    let project = project("fix-paths");
    let (exit, stderr) = klint(&project.0, &["--fix=safe", "src/a.ts"]);

    assert_eq!(exit, 0, "{stderr}");
    assert_eq!(source(&project.0, "a.ts"), LAYOUT_FIXED);
    assert_eq!(source(&project.0, "b.ts"), SOURCE);
}

#[test]
fn a_directory_path_covers_the_files_under_it() {
    let project = project("fix-dir");
    let (exit, stderr) = klint(&project.0, &["--fix", "src"]);

    assert_eq!(exit, 0, "{stderr}");
    assert_eq!(source(&project.0, "b.ts"), ALL_FIXED);
}

#[test]
fn a_missing_path_is_skipped_with_a_note_and_fixes_nothing() {
    let project = project("fix-missing");
    let (exit, stderr) = klint(&project.0, &["--fix", "src/gone.ts"]);

    assert_eq!(exit, 0, "{stderr}");
    assert!(
        stderr.starts_with("klint: skipping src/gone.ts:"),
        "{stderr}"
    );
    assert_eq!(source(&project.0, "a.ts"), SOURCE);
}

#[test]
fn an_unknown_fix_mode_is_refused() {
    let project = project("fix-mode");
    let (exit, stderr) = klint(&project.0, &["--fix=everything"]);

    assert_eq!(exit, 1);
    assert_eq!(
        stderr,
        "klint: unknown --fix mode 'everything', expected: safe\n"
    );
    assert_eq!(source(&project.0, "a.ts"), SOURCE);
}

#[test]
fn a_path_without_fix_is_refused() {
    let project = project("fix-path-only");
    let (exit, stderr) = klint(&project.0, &["src/a.ts"]);

    assert_eq!(exit, 1);
    assert_eq!(
        stderr,
        "klint: 'src/a.ts' is not a command, and file paths work only with --fix\n"
    );
}

#[test]
fn subcommands_still_dispatch() {
    let project = project("fix-stats");
    let (exit, stderr) = klint(&project.0, &["stats"]);

    assert_eq!(exit, 0, "{stderr}");
}
