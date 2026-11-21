mod common;

use common::temp_root;
use klint_rs::{RunOptions, run};
use std::fs::{create_dir_all, remove_dir_all, write};
use std::path::PathBuf;

const CONFIG: &str = "include: [\"src\"]\nrules:\n  core:\n    no-consecutive-array-push: error\n";

struct Project(PathBuf);

impl Drop for Project {
    fn drop(&mut self) {
        let _ = remove_dir_all(&self.0);
    }
}

fn findings(name: &str, source: &str) -> Vec<(usize, String)> {
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
        .filter(|violation| violation.rule == "no-consecutive-array-push")
        .map(|violation| (violation.line, violation.message))
        .collect()
}

fn message(count: usize, receiver: &str) -> String {
    format!(
        "{count} consecutive .push() calls on `{receiver}` — combine into a single .push(a, b, …) call."
    )
}

#[test]
fn a_new_receiver_closes_one_run_and_starts_the_next() {
    let source = "a.push(1);\na.push(2);\nb.push(3);\nb.push(4);\nb.push(5);\n";

    assert_eq!(
        findings("push-switch", source),
        vec![(1, message(2, "a")), (3, message(3, "b"))]
    );
}

#[test]
fn alternating_receivers_form_no_run() {
    let source = "a.push(1);\nb.push(2);\na.push(3);\nb.push(4);\n";

    assert!(findings("push-alternating", source).is_empty());
}

#[test]
fn runs_inside_function_bodies_and_on_member_receivers_are_found() {
    let source =
        "class List {\n  fill() {\n    this.items.push(1);\n    this.items.push(2);\n  }\n}\n";

    assert_eq!(
        findings("push-nested", source),
        vec![(3, message(2, "this.items"))]
    );
}
