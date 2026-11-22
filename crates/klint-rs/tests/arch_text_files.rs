mod common;

use common::temp_root;
use klint_rs::{RunOptions, run};
use std::fs::{create_dir_all, remove_dir_all, write};
use std::path::PathBuf;

struct Project(PathBuf);

impl Drop for Project {
    fn drop(&mut self) {
        let _ = remove_dir_all(&self.0);
    }
}

fn findings(name: &str, config: &str, files: &[(&str, &str)]) -> Vec<(String, usize, String)> {
    let project = Project(temp_root(name));

    for (file, text) in files {
        let path = project.0.join(file);
        create_dir_all(path.parent().expect("parent dir")).expect("create dirs");
        write(path, text).expect("write file");
    }

    write(project.0.join("klint.yaml"), config).expect("write config");
    let output = run(RunOptions {
        config_dir: project.0.clone(),
    })
    .expect("valid config runs");
    let mut found: Vec<(String, usize, String)> = output
        .violations
        .into_iter()
        .filter(|violation| violation.file != "klint.yaml")
        .map(|violation| (violation.file, violation.line, violation.rule))
        .collect();
    found.sort();
    found
}

fn found(file: &str, line: usize, rule: &str) -> (String, usize, String) {
    (file.to_string(), line, rule.to_string())
}

const PACKAGE: &str = "{\n  \"dependencies\": { \"x\": \"^1.0.0\" }\n}\n";

#[test]
fn forbidden_reaches_a_config_file_its_glob_names() {
    let config = "include: [\".\"]\nrules: {}\narch:\n  forbidden:\n    - {pattern: \"\\\"^\", in: \"**/package.json\", message: \"Pin exact versions\"}\n";

    assert_eq!(
        findings("text-forbidden", config, &[("package.json", PACKAGE)]),
        vec![found("package.json", 2, "arch/forbidden")]
    );
}

#[test]
fn max_lines_reaches_a_prisma_schema() {
    let config = "include: [\".\"]\nrules: {}\narch:\n  maxLines:\n    - {limit: 2, in: \"prisma/*.prisma\"}\n";
    let schema = "model User {\n  id Int @id\n}\n";

    assert_eq!(
        findings(
            "text-max-lines",
            config,
            &[("prisma/schema.prisma", schema)]
        ),
        vec![found("prisma/schema.prisma", 3, "arch/max-lines")]
    );
}

#[test]
fn singleton_reaches_a_named_config_file() {
    let config = "include: [\".\"]\nrules: {}\narch:\n  singleton:\n    - {pattern: \"DATABASE_URL\", only: \"config/app.yaml\", in: \"config/*.yaml\", message: \"Set it once\"}\n";

    assert_eq!(
        findings(
            "text-singleton",
            config,
            &[
                ("config/app.yaml", "DATABASE_URL: a\n"),
                ("config/test.yaml", "DATABASE_URL: b\n")
            ]
        ),
        vec![found("config/test.yaml", 1, "arch/singleton")]
    );
}

#[test]
fn a_bare_double_star_scope_stays_on_source_files() {
    let config = "include: [\".\"]\nrules: {}\narch:\n  forbidden:\n    - {pattern: \"TODO\", in: \"**\", message: \"No TODO\"}\n";

    assert_eq!(
        findings(
            "text-double-star",
            config,
            &[
                ("src/a.ts", "// TODO\n"),
                ("README.md", "TODO\n"),
                ("c.yaml", "a: TODO\n")
            ]
        ),
        vec![found("src/a.ts", 1, "arch/forbidden")]
    );
}

#[test]
fn a_named_text_file_is_never_parsed_as_source() {
    let config = "include: [\".\"]\nrules: {}\narch:\n  maxLines:\n    - {limit: 99, in: \"notes/*.txt\"}\n  maxCommentBlock:\n    - {limit: 1, in: \"notes/*.txt\"}\n";

    assert!(
        findings(
            "text-no-parse",
            config,
            &[("notes/a.txt", "// one\n// two\n// three\n")]
        )
        .is_empty()
    );
}

#[test]
fn include_still_limits_config_files() {
    let config = "include: [\"src\"]\nrules: {}\narch:\n  forbidden:\n    - {pattern: \"\\\"^\", in: \"**/package.json\", message: \"Pin exact versions\"}\n";

    assert!(findings("text-include", config, &[("package.json", PACKAGE)]).is_empty());
}
