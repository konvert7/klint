mod common;

use common::temp_root;
use klint_rs::{RunOptions, run};
use std::fs::{create_dir_all, remove_dir_all, write};
use std::path::PathBuf;

const CONFIG: &str = "include: [\"src\"]\nrules:\n  core:\n    no-unguarded-json-parse: error\n    no-sync-in-async: error\n";

struct Project(PathBuf);

impl Drop for Project {
    fn drop(&mut self) {
        let _ = remove_dir_all(&self.0);
    }
}

fn flagged_lines(name: &str, rule: &str, source: &str) -> Vec<usize> {
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
        .filter(|violation| violation.rule == rule)
        .map(|violation| violation.line)
        .collect()
}

#[test]
fn sync_calls_in_async_methods_callbacks_and_function_expressions_are_flagged() {
    let source = "class Store {\n  async load() {\n    fs.readFileSync(path);\n  }\n  save() {\n    writeFileSync(path);\n  }\n}\n[1].forEach(async () => readFileSync(path));\nfunction plain() {\n  return async function () {\n    mkdirSync(dir);\n  };\n}\n";

    assert_eq!(
        flagged_lines("sync-shapes", "no-sync-in-async", source),
        vec![3, 9, 12]
    );
}

#[test]
fn a_sync_arrow_inside_an_async_function_is_not_flagged() {
    let source =
        "async function outer() {\n  const read = () => readFileSync(path);\n  return read;\n}\n";

    assert!(flagged_lines("sync-nested-arrow", "no-sync-in-async", source).is_empty());
}

#[test]
fn optional_chained_json_parse_is_flagged() {
    let source = "const value = JSON?.parse(raw);\n";

    assert_eq!(
        flagged_lines("json-optional", "no-unguarded-json-parse", source),
        vec![1]
    );
}

#[test]
fn json_parse_in_a_try_block_of_a_nested_arrow_is_not_flagged() {
    let source = "const read = () => {\n  try {\n    return JSON.parse(raw);\n  } catch {\n    return null;\n  }\n};\n";

    assert!(flagged_lines("json-arrow-try", "no-unguarded-json-parse", source).is_empty());
}
