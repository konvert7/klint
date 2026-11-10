mod common;

use common::temp_root;
use serde_json::Value;
use std::fs::{create_dir_all, remove_dir_all, write};
use std::process::Command;

struct Run {
    exit: i32,
    stderr: String,
    findings: Vec<(String, u64, String)>,
}

fn lint(config: &str, files: &[(&str, &str)]) -> Run {
    let root = temp_root("arch-imports-e2e");
    for (relative, content) in files {
        let path = root.join(relative);
        create_dir_all(path.parent().expect("file has a parent")).expect("create dirs");
        write(path, content).expect("write source");
    }
    write(root.join("klint.yaml"), config).expect("write config");
    let output = Command::new(env!("CARGO_BIN_EXE_klint-rs"))
        .args(["--config", &root.to_string_lossy(), "--json"])
        .env("KLINT_NO_STATS", "1")
        .output()
        .expect("klint-rs runs");
    let _ = remove_dir_all(&root);
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout is JSON");
    Run {
        exit: output.status.code().expect("exits normally"),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        findings: json["violations"]
            .as_array()
            .expect("violations array")
            .iter()
            .map(|v| {
                (
                    v["file"].as_str().expect("file").to_string(),
                    v["line"].as_u64().expect("line"),
                    v["rule"].as_str().expect("rule").to_string(),
                )
            })
            .collect(),
    }
}

fn import_lines(run: &Run, file: &str) -> Vec<u64> {
    run.findings
        .iter()
        .filter(|(f, _, rule)| f == file && rule == "arch/imports")
        .map(|(_, line, _)| *line)
        .collect()
}

fn assert_import_errors(run: &Run, file: &str, lines: &[u64]) {
    assert_eq!(run.stderr, "");
    assert_eq!(run.exit, 2, "{:?}", run.findings);
    assert_eq!(run.findings.len(), lines.len(), "{:?}", run.findings);
    assert_eq!(import_lines(run, file), lines);
}

#[test]
fn swift_imports_with_attributes_and_submodule_paths_cross_layers() {
    let run = lint(
        "include: [\"Sources\"]\nrules: {}\narch:\n  layers:\n    ui: [\"Sources/App/UI/**\"]\n    core: [\"Sources/App/Core/**\"]\n  imports:\n    - from: ui\n      deny: core\n      message: \"UI must not import core directly\"\n",
        &[
            (
                "Sources/App/UI/ViewModel.swift",
                "@_exported import Core\n@testable import Core\nimport struct Core.Auth\nimport Core.Session\n",
            ),
            ("Sources/App/Core/Auth.swift", "public struct Auth {}\n"),
        ],
    );

    assert_import_errors(&run, "Sources/App/UI/ViewModel.swift", &[1, 2, 3, 4]);
}

#[test]
fn python_imports_resolve_namespace_packages_without_init_files() {
    let run = lint(
        "include: [\"src\"]\nrules: {}\narch:\n  layers:\n    jobs: [\"src/app/jobs/**\"]\n    lib: [\"src/app/lib/**\"]\n  imports:\n    - from: jobs\n      deny: lib\n      message: \"Jobs must not import lib directly\"\n",
        &[
            (
                "src/app/jobs/worker.py",
                "from app.lib import auth\nimport app.lib\n",
            ),
            ("src/app/lib/auth.py", "class Key:\n    pass\n"),
        ],
    );

    assert_import_errors(&run, "src/app/jobs/worker.py", &[1, 2]);
}

#[test]
fn python_deny_packages_matches_whole_names_including_dynamic_imports() {
    let run = lint(
        "include: [\"src\"]\nrules: {}\narch:\n  layers:\n    jobs: [\"src/app/jobs/**\"]\n  imports:\n    - from: jobs\n      deny-packages: [\"requests\", \"os\"]\n      message: \"Jobs must go through the http client and config\"\n",
        &[
            (
                "src/app/jobs/worker.py",
                "import requests\nimport os.path\nfrom requests.adapters import HTTPAdapter\nimport requests_mock\nimport oscrypto\nfrom . import helper\nimport importlib\nlazy = importlib.import_module(\"requests.sessions\")\n",
            ),
            ("src/app/jobs/helper.py", "value = 1\n"),
        ],
    );

    assert_import_errors(&run, "src/app/jobs/worker.py", &[1, 2, 3, 8]);
}

#[test]
fn rust_use_declarations_resolve_crate_paths_and_skip_comments() {
    let run = lint(
        "include: [\"crates\"]\nrules: {}\narch:\n  layers:\n    syntax: [\"crates/app/src/syntax/**\"]\n    output: [\"crates/app/src/output.rs\"]\n  imports:\n    - from: syntax\n      deny: output\n      message: \"Syntax must not depend on output\"\n",
        &[
            ("crates/app/src/lib.rs", "mod output;\npub mod syntax;\n"),
            ("crates/app/src/output.rs", "pub struct Violation;\n"),
            (
                "crates/app/src/syntax/mod.rs",
                "use crate::output::Violation;\n// use crate::output::Ignored;\nuse std::fs;\n",
            ),
        ],
    );

    assert_import_errors(&run, "crates/app/src/syntax/mod.rs", &[1]);
}

#[test]
fn rust_braced_use_lists_check_every_target() {
    let run = lint(
        "include: [\"crates\"]\nrules: {}\narch:\n  layers:\n    cli: [\"crates/app/src/cli.rs\"]\n    core: [\"crates/app/src/core/**\"]\n  imports:\n    - from: cli\n      deny: core\n      message: \"CLI must not reach into core internals\"\n",
        &[
            ("crates/app/src/lib.rs", "mod cli;\nmod core;\n"),
            (
                "crates/app/src/core/mod.rs",
                "pub mod parse;\npub mod render;\n",
            ),
            ("crates/app/src/core/parse.rs", "pub fn parse() {}\n"),
            ("crates/app/src/core/render.rs", "pub fn render() {}\n"),
            (
                "crates/app/src/cli.rs",
                "use crate::core::{parse::parse, render::render};\nuse std::io;\n",
            ),
        ],
    );

    assert_import_errors(&run, "crates/app/src/cli.rs", &[1, 1]);
}

#[test]
fn rust_self_and_super_paths_resolve_relative_to_the_module() {
    let run = lint(
        "include: [\"crates\"]\nrules: {}\narch:\n  layers:\n    leaf: [\"crates/app/src/feature/leaf.rs\"]\n    banned: [\"crates/app/src/banned.rs\", \"crates/app/src/feature/helper.rs\"]\n  imports:\n    - from: leaf\n      deny: banned\n      message: \"Leaf must not reach these modules\"\n",
        &[
            ("crates/app/src/lib.rs", "mod banned;\nmod feature;\n"),
            ("crates/app/src/banned.rs", "pub fn nope() {}\n"),
            (
                "crates/app/src/feature/mod.rs",
                "pub mod helper;\npub mod leaf;\n",
            ),
            ("crates/app/src/feature/helper.rs", "pub fn help() {}\n"),
            (
                "crates/app/src/feature/leaf.rs",
                "use super::helper::help;\nuse crate::banned::nope;\nuse std::fmt;\n",
            ),
        ],
    );

    assert_import_errors(&run, "crates/app/src/feature/leaf.rs", &[1, 2]);
}

#[test]
fn rust_deny_packages_matches_crates_and_paths_by_segment() {
    let run = lint(
        "include: [\"crates\"]\nrules: {}\narch:\n  layers:\n    core: [\"crates/app/src/core.rs\"]\n  imports:\n    - from: core\n      deny-packages: [\"tokio\", \"std::process\"]\n      message: \"Core must stay runtime-free and must not spawn processes\"\n",
        &[
            ("crates/app/src/lib.rs", "mod core;\n"),
            (
                "crates/app/src/core.rs",
                "use tokio::runtime::Runtime;\nuse std::process::Command;\nuse std::fs;\nuse tokiox::thing;\n",
            ),
        ],
    );

    assert_import_errors(&run, "crates/app/src/core.rs", &[1, 2]);
}

#[test]
fn typescript_re_exports_cross_layers_unless_type_only() {
    let run = lint(
        "include: [\".\"]\nrules: {}\narch:\n  layers:\n    skills: [\"assets/skills/**\"]\n    core: [\"src/lib/**\"]\n  imports:\n    - from: skills\n      deny: core\n      type-only: allow\n      message: \"Skills must be self-contained\"\n",
        &[
            (
                "assets/skills/demo/index.ts",
                "export { foo } from \"../../../src/lib/utils\";\nexport * from \"../../../src/lib/dynamic\";\nexport type { Shape } from \"../../../src/lib/types\";\nexport const local = 1;\n",
            ),
            ("src/lib/utils.ts", "export const foo = 1;\n"),
            ("src/lib/dynamic.ts", "export const dynamic = 1;\n"),
            ("src/lib/types.ts", "export type Shape = { id: string };\n"),
        ],
    );

    assert_import_errors(&run, "assets/skills/demo/index.ts", &[1, 2]);
}

#[test]
fn typescript_deny_packages_blocks_a_re_exported_package() {
    let run = lint(
        "include: [\".\"]\nrules: {}\narch:\n  layers:\n    dao: [\"src/dao/**\"]\n  imports:\n    - from: dao\n      deny-packages: [\"next/headers\"]\n      message: \"Data access must not read request state\"\n",
        &[(
            "src/dao/user.ts",
            "export { headers } from \"next/headers\";\nexport { cache } from \"react\";\n",
        )],
    );

    assert_import_errors(&run, "src/dao/user.ts", &[1]);
}

#[test]
fn a_stale_schema_alone_warns_on_its_own_line_without_failing() {
    let run = lint(
        "\n$schema: https://raw.githubusercontent.com/konvert7/klint/refs/tags/v0.0.1/klint.schema.json\ninclude: [\"src\"]\nrules: {}\narch:\n  maxLines:\n    - limit: 500\n      in: \"src/**\"\n",
        &[("src/subject.ts", "export const a = 1;\n")],
    );

    assert_eq!(run.exit, 0);
    assert_eq!(
        run.findings,
        vec![(
            "klint.yaml".to_string(),
            2,
            "klint/schema-version".to_string()
        )]
    );
}
