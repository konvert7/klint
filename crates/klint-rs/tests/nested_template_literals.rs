mod common;

use common::temp_root;
use klint_rs::{RunOptions, run};
use std::fs::{create_dir_all, remove_dir_all, write};
use std::path::PathBuf;

const CONFIG: &str =
    "include: [\"src\"]\nrules:\n  core:\n    no-nested-template-literals: error\n";

struct Project(PathBuf);

impl Drop for Project {
    fn drop(&mut self) {
        let _ = remove_dir_all(&self.0);
    }
}

fn flagged_columns(name: &str, source: &str) -> Vec<(usize, Option<usize>)> {
    let project = Project(temp_root(name));
    create_dir_all(project.0.join("src")).expect("create src");
    write(project.0.join("klint.yaml"), CONFIG).expect("write config");
    write(project.0.join("src/a.ts"), source).expect("write source");
    let output = run(RunOptions {
        config_dir: project.0.clone(),
    })
    .expect("valid config runs");

    output
        .violations
        .into_iter()
        .filter(|violation| violation.rule == "no-nested-template-literals")
        .map(|violation| (violation.line, violation.span.map(|span| span.column)))
        .collect()
}

#[test]
fn every_level_of_a_deep_nesting_is_reported() {
    let source = "const a = `x ${`y ${`z`}`}`;\n";

    assert_eq!(
        flagged_columns("template-deep", source),
        vec![(1, Some(16)), (1, Some(21))]
    );
}

#[test]
fn a_template_inside_a_callback_in_a_substitution_is_reported() {
    let source = "const b = `x ${[1].map((n) => `${n}`).join()}`;\n";

    assert_eq!(
        flagged_columns("template-callback", source),
        vec![(1, Some(31))]
    );
}
