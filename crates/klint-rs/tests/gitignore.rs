mod common;

use common::temp_root;
use klint_rs::{RunOptions, run};
use std::fs::{create_dir_all, remove_dir_all, write};
use std::path::{Path, PathBuf};
use std::process::Command;

const OFFENDING: &str = "console.log(\"x\");\n";

struct Project(PathBuf);

impl Drop for Project {
    fn drop(&mut self) {
        let _ = remove_dir_all(&self.0);
    }
}

fn project(name: &str, config: &str, files: &[(&str, &str)]) -> Project {
    let root = temp_root(name);

    for (path, text) in files {
        let target = root.join(path);
        create_dir_all(target.parent().expect("parent dir")).expect("create dirs");
        write(target, text).expect("write file");
    }

    write(root.join("klint.yaml"), config).expect("write config");
    Project(root)
}

fn as_repository(project: &Project) {
    create_dir_all(project.0.join(".git/info")).expect("create .git");
}

fn config(extra: &str) -> String {
    format!(
        "{extra}include: [\".\"]\nrules: {{}}\narch:\n  forbidden:\n    - pattern: \"console.log(\"\n      in: \"**\"\n      message: \"Use logger\"\n"
    )
}

fn flagged_files(root: &Path) -> Vec<String> {
    let output = run(RunOptions {
        config_dir: root.to_path_buf(),
    })
    .expect("valid config runs");
    let mut files: Vec<String> = output
        .violations
        .into_iter()
        .filter(|violation| violation.rule == "arch/forbidden")
        .map(|violation| violation.file)
        .collect();
    files.sort();
    files
}

#[test]
fn an_ignored_directory_yields_no_findings() {
    let project = project(
        "gitignore-dir",
        &config(""),
        &[
            (".gitignore", "dist/\n"),
            ("src/a.ts", OFFENDING),
            ("dist/a.js", OFFENDING),
        ],
    );
    as_repository(&project);

    assert_eq!(flagged_files(&project.0), vec!["src/a.ts"]);
}

#[test]
fn nested_gitignore_files_and_info_exclude_apply() {
    let project = project(
        "gitignore-nested",
        &config(""),
        &[
            ("pkg/.gitignore", "build/\n"),
            ("pkg/src/a.ts", OFFENDING),
            ("pkg/build/a.js", OFFENDING),
            ("scratch/a.ts", OFFENDING),
        ],
    );
    as_repository(&project);
    write(project.0.join(".git/info/exclude"), "scratch/\n").expect("write exclude");

    assert_eq!(flagged_files(&project.0), vec!["pkg/src/a.ts"]);
}

#[test]
fn a_directory_outside_git_still_lints_everything() {
    let project = project(
        "gitignore-no-git",
        &config(""),
        &[
            (".gitignore", "dist/\n"),
            ("src/a.ts", OFFENDING),
            ("dist/a.js", OFFENDING),
        ],
    );

    assert_eq!(flagged_files(&project.0), vec!["dist/a.js", "src/a.ts"]);
}

#[test]
fn include_patterns_keep_selecting_and_excluding() {
    let project = project(
        "gitignore-include",
        "include: [\"src\", \"!src/gen/**\"]\nrules: {}\narch:\n  forbidden:\n    - pattern: \"console.log(\"\n      in: \"**\"\n      message: \"Use logger\"\n",
        &[
            (".gitignore", "*.local.ts\n"),
            ("src/a.ts", OFFENDING),
            ("src/b.local.ts", OFFENDING),
            ("src/gen/c.ts", OFFENDING),
            ("other/d.ts", OFFENDING),
        ],
    );
    as_repository(&project);

    assert_eq!(flagged_files(&project.0), vec!["src/a.ts"]);
}

#[test]
fn gitignore_wins_over_an_explicit_include() {
    let project = project(
        "gitignore-wins",
        "include: [\"dist/a.js\"]\nrules: {}\narch:\n  forbidden:\n    - pattern: \"console.log(\"\n      in: \"**\"\n      message: \"Use logger\"\n",
        &[(".gitignore", "dist/\n"), ("dist/a.js", OFFENDING)],
    );
    as_repository(&project);

    assert!(flagged_files(&project.0).is_empty());
}

#[test]
fn respect_gitignore_false_lints_ignored_files() {
    let project = project(
        "gitignore-off",
        &config("respectGitignore: false\n"),
        &[
            (".gitignore", "dist/\n"),
            ("src/a.ts", OFFENDING),
            ("dist/a.js", OFFENDING),
        ],
    );
    as_repository(&project);

    assert_eq!(flagged_files(&project.0), vec!["dist/a.js", "src/a.ts"]);
}

#[test]
fn a_relative_config_dir_still_finds_the_files() {
    let project = project(
        "gitignore-relative",
        &config("").replace("include: [\".\"]", "include: [\".\", \"!src/gen/b.ts\"]"),
        &[
            (".gitignore", "dist/\n"),
            ("src/a.ts", OFFENDING),
            ("src/gen/b.ts", OFFENDING),
            ("dist/a.js", OFFENDING),
        ],
    );
    as_repository(&project);
    let output = Command::new(env!("CARGO_BIN_EXE_klint-rs"))
        .current_dir(&project.0)
        .args(["--config", ".", "--json"])
        .env("KLINT_NO_STATS", "1")
        .output()
        .expect("klint-rs runs");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(output.status.code(), Some(2), "{stdout}");
    assert!(stdout.contains("\"file\":\"src/a.ts\""), "{stdout}");
    assert!(!stdout.contains("dist/a.js"), "{stdout}");
    assert!(!stdout.contains("src/gen/b.ts"), "{stdout}");
}
