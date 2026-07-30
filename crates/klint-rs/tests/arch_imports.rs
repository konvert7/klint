mod common;

use common::temp_root;
use klint_rs::{RunOptions, run};
use std::fs;
use std::fs::{create_dir_all, write};

#[test]
fn imports_deny_mode_flags_static_and_dynamic_relative_imports() {
    let root = temp_root("imports-deny-relative");
    create_dir_all(root.join("assets/skills/demo")).expect("create skill dirs");
    create_dir_all(root.join("src/lib")).expect("create core dirs");
    write(
        root.join("klint.yaml"),
        r#"
include: ["."]
rules: {}
arch:
  layers:
    skills: ["assets/skills/**"]
    core: ["src/lib/**"]
  imports:
    - from: skills
      deny: core
      message: "Skills must be self-contained"
"#,
    )
    .expect("write config");
    write(
        root.join("assets/skills/demo/index.ts"),
        "import { foo } from \"../../../src/lib/utils\";\nexport async function load() {\n  return import(\"../../../src/lib/dynamic\");\n}\n",
    )
    .expect("write importing source");
    write(root.join("src/lib/utils.ts"), "export const foo = 1;\n").expect("write util");
    write(
        root.join("src/lib/dynamic.ts"),
        "export const dynamic = 1;\n",
    )
    .expect("write dynamic util");

    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");

    assert_eq!(output.summary.errors, 2);
    assert_eq!(
        output
            .violations
            .iter()
            .map(|violation| violation.line)
            .collect::<Vec<_>>(),
        vec![1, 3]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn imports_deny_mode_can_allow_type_only_imports() {
    let root = temp_root("imports-type-only-allow");
    create_dir_all(root.join("assets/skills/demo")).expect("create skill dirs");
    create_dir_all(root.join("src/lib")).expect("create core dirs");
    write(
        root.join("klint.yaml"),
        r#"
include: ["."]
rules: {}
arch:
  layers:
    skills: ["assets/skills/**"]
    core: ["src/lib/**"]
  imports:
    - from: skills
      deny: core
      type-only: allow
      message: "Use runtime boundary"
"#,
    )
    .expect("write config");
    write(
        root.join("assets/skills/demo/index.ts"),
        "import type { Foo } from \"../../../src/lib/types\";\nimport { foo } from \"../../../src/lib/utils\";\nexport const value = foo;\n",
    )
    .expect("write importing source");
    write(
        root.join("src/lib/types.ts"),
        "export interface Foo { value: string }\n",
    )
    .expect("write types");
    write(root.join("src/lib/utils.ts"), "export const foo = 1;\n").expect("write util");

    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");

    assert_eq!(output.summary.errors, 1);
    assert_eq!(output.violations.len(), 1);
    assert_eq!(output.violations[0].line, 2);
    assert_eq!(output.violations[0].message, "Use runtime boundary");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn imports_deny_mode_flags_type_only_imports_without_override() {
    let root = temp_root("imports-type-only-default-deny");
    create_dir_all(root.join("assets/skills/demo")).expect("create skill dirs");
    create_dir_all(root.join("src/lib")).expect("create core dirs");
    write(
        root.join("klint.yaml"),
        r#"
include: ["."]
rules: {}
arch:
  layers:
    skills: ["assets/skills/**"]
    core: ["src/lib/**"]
  imports:
    - from: skills
      deny: core
      message: "Use runtime boundary"
"#,
    )
    .expect("write config");
    write(
        root.join("assets/skills/demo/index.ts"),
        "import type { Foo } from \"../../../src/lib/types\";\n",
    )
    .expect("write importing source");
    write(
        root.join("src/lib/types.ts"),
        "export interface Foo { value: string }\n",
    )
    .expect("write types");

    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");

    assert_eq!(output.summary.errors, 1);
    assert_eq!(output.violations[0].line, 1);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn imports_deny_mode_resolves_tsconfig_path_aliases() {
    let root = temp_root("imports-path-alias");
    create_dir_all(root.join("assets/skills/demo")).expect("create skill dirs");
    create_dir_all(root.join("src/lib")).expect("create core dirs");
    write(
        root.join("tsconfig.json"),
        r#"
{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@/*": ["src/*"]
    }
  }
}
"#,
    )
    .expect("write tsconfig");
    write(
        root.join("klint.yaml"),
        r#"
include: ["."]
rules: {}
arch:
  layers:
    skills: ["assets/skills/**"]
    core: ["src/**"]
  imports:
    - from: skills
      deny: core
      message: "No repo source imports from skills"
      severity: warn
"#,
    )
    .expect("write config");
    write(
        root.join("assets/skills/demo/index.ts"),
        "import { foo } from \"@/lib/utils\";\nexport const value = foo;\n",
    )
    .expect("write importing source");
    write(root.join("src/lib/utils.ts"), "export const foo = 1;\n").expect("write util");

    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");

    assert_eq!(output.summary.errors, 0);
    assert_eq!(output.summary.warnings, 1);
    assert_eq!(output.violations[0].line, 1);
    assert_eq!(output.violations[0].severity, "warn");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn imports_deny_mode_resolves_exact_tsconfig_path_aliases() {
    let root = temp_root("imports-exact-path-alias");
    create_dir_all(root.join("assets/skills/demo")).expect("create skill dirs");
    create_dir_all(root.join("src/lib")).expect("create core dirs");
    write(
        root.join("tsconfig.json"),
        r#"
{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@core": ["src/lib/index.ts"]
    }
  }
}
"#,
    )
    .expect("write tsconfig");
    write(
        root.join("klint.yaml"),
        r#"
include: ["."]
rules: {}
arch:
  layers:
    skills: ["assets/skills/**"]
    core: ["src/lib/index.ts"]
  imports:
    - from: skills
      deny: core
      message: "No repo source imports from skills"
"#,
    )
    .expect("write config");
    write(
        root.join("assets/skills/demo/index.ts"),
        "import { foo } from \"@core\";\n",
    )
    .expect("write importing source");
    write(root.join("src/lib/index.ts"), "export const foo = 1;\n").expect("write core");

    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");

    assert_eq!(output.summary.errors, 1);
    assert_eq!(output.violations[0].line, 1);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn imports_allow_mode_flags_imports_outside_allowlist() {
    let root = temp_root("imports-allow-mode-blocks-unlisted");
    create_dir_all(root.join("src/dao")).expect("create dao dirs");
    create_dir_all(root.join("src/prisma")).expect("create prisma dirs");
    create_dir_all(root.join("src/service")).expect("create service dirs");
    write(
        root.join("klint.yaml"),
        r#"
include: ["."]
rules: {}
arch:
  imports:
    - from: ["src/dao/**"]
      allow: ["src/dao/**", "src/prisma/**"]
      message: "DAO may only import from dao or prisma"
"#,
    )
    .expect("write config");
    write(
        root.join("src/dao/user.ts"),
        "import { db } from \"../prisma/client\";\nimport { service } from \"../service/user\";\n",
    )
    .expect("write dao source");
    write(root.join("src/prisma/client.ts"), "export const db = {};\n")
        .expect("write prisma source");
    write(
        root.join("src/service/user.ts"),
        "export const service = {};\n",
    )
    .expect("write service source");

    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");

    assert_eq!(output.summary.errors, 1);
    assert_eq!(output.violations.len(), 1);
    assert_eq!(output.violations[0].line, 2);
    assert_eq!(
        output.violations[0].message,
        "DAO may only import from dao or prisma"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn imports_allow_mode_skips_bare_package_imports() {
    let root = temp_root("imports-allow-mode-skips-packages");
    create_dir_all(root.join("src/dao")).expect("create dao dirs");
    write(
        root.join("klint.yaml"),
        r#"
include: ["."]
rules: {}
arch:
  imports:
    - from: ["src/dao/**"]
      allow: ["src/dao/**"]
"#,
    )
    .expect("write config");
    write(root.join("src/dao/user.ts"), "import { z } from \"zod\";\n").expect("write dao source");

    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");

    assert_eq!(output.summary.errors, 0);
    assert_eq!(output.violations.len(), 0);
    let _ = fs::remove_dir_all(root);
}
