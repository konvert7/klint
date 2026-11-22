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

type Finding = (String, usize, Option<usize>, String);

fn findings(name: &str, arch: &str, files: &[(&str, &str)]) -> Vec<Finding> {
    let project = Project(temp_root(name));

    for (file, text) in files {
        let path = project.0.join(file);
        create_dir_all(path.parent().expect("parent dir")).expect("create dirs");
        write(path, text).expect("write file");
    }

    write(
        project.0.join("klint.yaml"),
        format!("include: [\".\"]\nrules: {{}}\narch:\n{arch}"),
    )
    .expect("write config");
    let output = run(RunOptions {
        config_dir: project.0.clone(),
    })
    .expect("valid config runs");

    output
        .violations
        .into_iter()
        .filter(|violation| violation.file != "klint.yaml")
        .map(|violation| {
            let column = violation.span.map(|span| span.column);
            (violation.file, violation.line, column, violation.rule)
        })
        .collect()
}

fn found(file: &str, line: usize, column: Option<usize>, rule: &str) -> Finding {
    (file.to_string(), line, column, rule.to_string())
}

const PIN: &str = "  forbidden:\n    - {key: \"dependencies.*\", pattern: \"re:^[\\\\^~]\", in: \"**/package.json\", message: \"Pin exact versions\"}\n";

#[test]
fn forbidden_value_is_reported_at_its_key_in_jsonc() {
    let package = "{\n  // deps\n  \"dependencies\": { \"a\": \"^1.0.0\", \"b\": \"2.0.0\" }\n}\n";

    assert_eq!(
        findings("keys-forbidden", PIN, &[("package.json", package)]),
        vec![found("package.json", 3, Some(21), "arch/forbidden")]
    );
}

#[test]
fn forbidden_key_without_a_pattern_forbids_the_key() {
    let arch = "  forbidden:\n    - {key: \"scripts.postinstall\", in: \"**/package.json\", message: \"No install scripts\"}\n";
    let package = "{\n  \"scripts\": {\n    \"postinstall\": \"node x.js\"\n  }\n}\n";

    assert_eq!(
        findings("keys-forbidden-key", arch, &[("package.json", package)]),
        vec![found("package.json", 3, Some(5), "arch/forbidden")]
    );
}

#[test]
fn required_reports_a_wrong_value_at_its_key_and_a_missing_key_on_line_one() {
    let arch = "  required:\n    - {key: package.edition, pattern: \"2024\", in: Cargo.toml, message: \"Use 2024\"}\n    - {key: package.rust-version, in: Cargo.toml, message: \"Set rust-version\"}\n";
    let cargo = "[package]\nedition = \"2021\"\n";

    assert_eq!(
        findings("keys-required", arch, &[("Cargo.toml", cargo)]),
        vec![
            found("Cargo.toml", 2, Some(1), "arch/required"),
            found("Cargo.toml", 1, None, "arch/required"),
        ]
    );
}

#[test]
fn required_is_silent_when_the_key_and_value_are_right() {
    let arch = "  required:\n    - {key: compilerOptions.strict, pattern: \"true\", in: tsconfig.json, message: \"Strict\"}\n";
    let tsconfig = "{\n  \"compilerOptions\": { \"strict\": true }\n}\n";

    assert!(findings("keys-required-ok", arch, &[("tsconfig.json", tsconfig)]).is_empty());
}

#[test]
fn a_star_reaches_into_yaml_mappings() {
    let arch = "  forbidden:\n    - {key: \"services.*.image\", pattern: \"re::latest$\", in: \"*.yaml\", message: \"Pin tags\"}\n";
    let compose = "services:\n  web:\n    image: node:latest\n  db:\n    image: postgres:17\n";

    assert_eq!(
        findings("keys-yaml", arch, &[("compose.yaml", compose)]),
        vec![found("compose.yaml", 3, Some(5), "arch/forbidden")]
    );
}

#[test]
fn a_literal_pattern_must_equal_the_value() {
    let arch = "  forbidden:\n    - {key: \"version\", pattern: \"1\", in: \"*.json\", message: \"No v1\"}\n";

    assert!(
        findings(
            "keys-literal",
            arch,
            &[("a.json", "{\"version\": \"10\"}\n")]
        )
        .is_empty()
    );
}

#[test]
fn a_parent_key_counts_as_present_through_its_children() {
    let arch =
        "  required:\n    - {key: dependencies, in: Cargo.toml, message: \"Needs dependencies\"}\n";
    let cargo = "[dependencies.serde]\nversion = \"1\"\n";

    assert!(findings("keys-parent", arch, &[("Cargo.toml", cargo)]).is_empty());
}

#[test]
fn a_key_rule_never_reads_source_files_or_lines() {
    let arch =
        "  forbidden:\n    - {key: \"x\", pattern: \"re:x\", in: \"**\", message: \"No x\"}\n";

    assert!(findings("keys-source", arch, &[("src/a.ts", "const x = 1;\n")]).is_empty());
}

#[test]
fn a_file_that_does_not_parse_is_skipped() {
    let arch = "  required:\n    - {key: compilerOptions, in: tsconfig.json, message: \"Needs options\"}\n";

    assert!(findings("keys-broken", arch, &[("tsconfig.json", "{\"a\": }\n")]).is_empty());
}

const EXACT_VERSIONS: &str = "  required:\n    - key: [\"*Dependencies.*\", \"overrides.**\", \"!optionalDependencies.@konvert7/klint-*\"]\n      pattern: \"re:^\\\\d+\\\\.\\\\d+\\\\.\\\\d+(-[0-9A-Za-z.-]+)?(\\\\+[0-9A-Za-z.-]+)?$\"\n      in: \"**/package.json\"\n      message: \"Pin exact versions\"\n";

#[test]
fn the_exact_versions_check_is_one_required_entry() {
    let package = "{\n  \"devDependencies\": {\n    \"zod\": \"^3.0.0\",\n    \"bun\": \"1.3.14\"\n  },\n  \"optionalDependencies\": {\n    \"@konvert7/klint-linux-x64\": \"*\",\n    \"left-pad\": \"*\"\n  },\n  \"overrides\": {\n    \"eslint\": { \"minimatch\": \"~9.0.0\" }\n  }\n}\n";
    let bare = "{\n  \"name\": \"native\"\n}\n";

    assert_eq!(
        findings(
            "keys-exact",
            EXACT_VERSIONS,
            &[("package.json", package), ("npm/native/package.json", bare)]
        ),
        vec![
            found("package.json", 3, Some(5), "arch/required"),
            found("package.json", 8, Some(5), "arch/required"),
            found("package.json", 11, Some(17), "arch/required"),
        ]
    );
}

#[test]
fn a_key_pass_checks_only_the_files_its_in_glob_names() {
    let arch = "  forbidden:\n    - {key: scripts.postinstall, in: \"**/package.json\", message: \"No install scripts\"}\n  required:\n    - {key: compilerOptions.strict, in: tsconfig.json, message: \"Strict\"}\n";
    let package = "{\n  \"name\": \"app\"\n}\n";
    let tsconfig = "{\n  \"compilerOptions\": { \"strict\": true }\n}\n";

    assert_eq!(
        findings(
            "keys-scoped",
            arch,
            &[("package.json", package), ("tsconfig.json", tsconfig)]
        ),
        Vec::<Finding>::new()
    );
}
