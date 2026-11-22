mod common;

use common::temp_root;
use klint_rs::{RunOptions, run};
use std::fs::{create_dir_all, remove_dir_all, write};
use std::path::PathBuf;

const OFFENDING: &str = "console.log(1);\n";

struct Project(PathBuf);

impl Drop for Project {
    fn drop(&mut self) {
        let _ = remove_dir_all(&self.0);
    }
}

fn flagged_files(name: &str, arch: &str, files: &[&str]) -> Vec<String> {
    let project = Project(temp_root(name));

    for file in files {
        let path = project.0.join(file);
        create_dir_all(path.parent().expect("parent dir")).expect("create dirs");
        write(path, OFFENDING).expect("write source");
    }

    write(
        project.0.join("klint.yaml"),
        format!("include: [\"src\"]\nrules: {{}}\narch:\n{arch}"),
    )
    .expect("write config");
    let output = run(RunOptions {
        config_dir: project.0.clone(),
    })
    .expect("valid config runs");
    let mut flagged: Vec<String> = output
        .violations
        .into_iter()
        .filter(|violation| violation.rule == "arch/forbidden")
        .map(|violation| violation.file)
        .collect();
    flagged.sort();
    flagged
}

const FILES: &[&str] = &[
    "src/a.ts",
    "src/a.test.ts",
    "src/lib/b.ts",
    "src/lib/b.test.ts",
];

fn forbidden_in(scope: &str) -> String {
    format!("  forbidden:\n    - {{pattern: \"console.log\", in: {scope}, message: \"no log\"}}\n")
}

#[test]
fn a_file_glob_in_scope_matches_only_those_files() {
    assert_eq!(
        flagged_files("in-file-glob", &forbidden_in("\"src/**/*.test.ts\""), FILES),
        vec!["src/a.test.ts", "src/lib/b.test.ts"]
    );
}

#[test]
fn a_directory_scope_still_covers_everything_below_it() {
    assert_eq!(
        flagged_files("in-dir", &forbidden_in("\"src/lib\""), FILES),
        vec!["src/lib/b.test.ts", "src/lib/b.ts"]
    );
}

#[test]
fn a_layer_made_of_file_globs_matches_only_those_files() {
    let arch = format!(
        "  layers:\n    tests: [\"src/**/*.test.ts\"]\n{}",
        forbidden_in("tests")
    );

    assert_eq!(
        flagged_files("in-layer-glob", &arch, FILES),
        vec!["src/a.test.ts", "src/lib/b.test.ts"]
    );
}
