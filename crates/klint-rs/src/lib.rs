mod arch;
mod config;
mod files;
mod output;
pub mod syntax;

use std::path::PathBuf;

use arch::run_arch_rules;
use config::{find_config, read_config, resolve_root};
use files::{read_files, resolve_files};
pub use output::{JsonOutput, Summary, Violation};

#[derive(Debug)]
pub struct RunOptions {
    pub config_dir: PathBuf,
}

pub fn run(options: RunOptions) -> Result<JsonOutput, String> {
    let config_path = find_config(&options.config_dir)?;
    let raw = read_config(&config_path)?;
    let root = resolve_root(&options.config_dir, raw.root.as_deref());
    let include = raw.include.unwrap_or_else(|| vec![".".to_string()]);

    let files = resolve_files(&root, &include)?;
    let file_contents = read_files(&files)?;
    let _plugins = raw.plugins.unwrap_or_default();
    let _rules = raw.rules.unwrap_or_default();

    let mut violations = Vec::new();
    if let Some(arch) = raw.arch {
        run_arch_rules(&arch, &files, &file_contents, &root, &mut violations);
    }

    Ok(output::output_from_violations(violations))
}

pub fn empty_output() -> JsonOutput {
    output::output_from_violations(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::fs::{create_dir_all, write};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic enough for tests")
            .as_nanos();
        std::env::temp_dir().join(format!("klint-rs-{name}-{id}"))
    }

    #[test]
    fn emits_empty_json_for_valid_yaml_config() {
        let root = temp_root("empty-yaml");
        create_dir_all(root.join("src")).expect("create fixture dirs");
        write(root.join("klint.yaml"), "include: [\"src\"]\nrules: {}\n").expect("write config");
        write(root.join("src/index.ts"), "export const value = 1;\n").expect("write source");

        let output = run(RunOptions {
            config_dir: root.clone(),
        })
        .expect("valid config should run");

        assert_eq!(output, empty_output());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prefers_yaml_over_json_config() {
        let root = temp_root("config-priority");
        create_dir_all(&root).expect("create fixture root");
        write(root.join("klint.yaml"), "include: [\"src\"]\nrules: {}\n")
            .expect("write yaml config");
        write(root.join("klint.config.json"), "{").expect("write broken json config");

        let output = run(RunOptions {
            config_dir: root.clone(),
        })
        .expect("yaml should be selected before json");

        assert_eq!(output, empty_output());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn missing_config_is_an_error() {
        let root = temp_root("missing-config");
        create_dir_all(&root).expect("create fixture root");

        let err = run(RunOptions {
            config_dir: root.clone(),
        })
        .expect_err("missing config should fail");

        assert!(err.contains("no config file found"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn forbidden_pattern_reports_matching_line() {
        let root = temp_root("forbidden-pattern");
        create_dir_all(root.join("src/lib")).expect("create fixture dirs");
        write(
            root.join("klint.yaml"),
            r#"
include: ["src"]
rules: {}
arch:
  layers:
    lib: ["src/lib/**"]
  forbidden:
    - pattern: "console.log("
      in: lib
      message: "Use logger"
"#,
        )
        .expect("write config");
        write(
            root.join("src/lib/utils.ts"),
            "export function debug() {\n  console.log(\"x\");\n}\n",
        )
        .expect("write source");

        let output = run(RunOptions {
            config_dir: root.clone(),
        })
        .expect("valid config should run");

        assert_eq!(output.summary.errors, 1);
        assert_eq!(output.summary.warnings, 0);
        assert_eq!(
            output.violations,
            vec![Violation {
                file: "src/lib/utils.ts".to_string(),
                line: 2,
                rule: "arch/forbidden".to_string(),
                message: "Use logger".to_string(),
                severity: "error".to_string(),
                fix: None,
            }]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn forbidden_pattern_respects_scope_and_severity() {
        let root = temp_root("forbidden-scope-severity");
        create_dir_all(root.join("src/lib")).expect("create lib dirs");
        create_dir_all(root.join("src/scripts")).expect("create scripts dirs");
        write(
            root.join("klint.yaml"),
            r#"
include: ["src"]
rules: {}
arch:
  forbidden:
    - pattern: "console.log("
      in: "src/lib/**"
      message: "Use logger"
      severity: warn
"#,
        )
        .expect("write config");
        write(root.join("src/lib/utils.ts"), "console.log(\"x\");\n").expect("write scoped source");
        write(root.join("src/scripts/debug.ts"), "console.log(\"x\");\n")
            .expect("write unscoped source");

        let output = run(RunOptions {
            config_dir: root.clone(),
        })
        .expect("valid config should run");

        assert_eq!(output.summary.errors, 0);
        assert_eq!(output.summary.warnings, 1);
        assert_eq!(output.violations.len(), 1);
        assert_eq!(output.violations[0].file, "src/lib/utils.ts");
        assert_eq!(output.violations[0].severity, "warn");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn singleton_pattern_ignores_only_file_and_reports_other_matches() {
        let root = temp_root("singleton-pattern");
        create_dir_all(root.join("src/lib")).expect("create lib dirs");
        create_dir_all(root.join("src/server")).expect("create server dirs");
        write(
            root.join("klint.yaml"),
            r#"
include: ["src"]
rules: {}
arch:
  singleton:
    - pattern: "process.env.API_KEY"
      only: "src/lib/auth.ts"
      in: ["src/**"]
      message: "Use auth module"
"#,
        )
        .expect("write config");
        write(
            root.join("src/lib/auth.ts"),
            "export const key = process.env.API_KEY;\n",
        )
        .expect("write allowed source");
        write(
            root.join("src/server/handler.ts"),
            "const key = process.env.API_KEY;\n",
        )
        .expect("write violating source");

        let output = run(RunOptions {
            config_dir: root.clone(),
        })
        .expect("valid config should run");

        assert_eq!(
            output.violations,
            vec![Violation {
                file: "src/server/handler.ts".to_string(),
                line: 1,
                rule: "arch/singleton".to_string(),
                message: "Use auth module".to_string(),
                severity: "error".to_string(),
                fix: None,
            }]
        );
        assert_eq!(output.summary.errors, 1);
        assert_eq!(output.summary.warnings, 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn singleton_pattern_respects_default_scope_and_warn_severity() {
        let root = temp_root("singleton-default-scope");
        create_dir_all(root.join("src/lib")).expect("create lib dirs");
        create_dir_all(root.join("src/app")).expect("create app dirs");
        write(
            root.join("klint.yaml"),
            r#"
include: ["src"]
rules: {}
arch:
  singleton:
    - pattern: "createClient("
      only: "src/lib/client.ts"
      message: "Use shared client"
      severity: warn
"#,
        )
        .expect("write config");
        write(root.join("src/lib/client.ts"), "createClient();\n").expect("write allowed source");
        write(root.join("src/app/page.ts"), "createClient();\n").expect("write violating source");

        let output = run(RunOptions {
            config_dir: root.clone(),
        })
        .expect("valid config should run");

        assert_eq!(output.summary.errors, 0);
        assert_eq!(output.summary.warnings, 1);
        assert_eq!(output.violations.len(), 1);
        assert_eq!(output.violations[0].file, "src/app/page.ts");
        assert_eq!(output.violations[0].severity, "warn");
        let _ = fs::remove_dir_all(root);
    }

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
        write(root.join("src/dao/user.ts"), "import { z } from \"zod\";\n")
            .expect("write dao source");

        let output = run(RunOptions {
            config_dir: root.clone(),
        })
        .expect("valid config should run");

        assert_eq!(output.summary.errors, 0);
        assert_eq!(output.violations.len(), 0);
        let _ = fs::remove_dir_all(root);
    }
}
