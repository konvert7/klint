mod common;

use common::temp_root;
use klint_rs::{RunOptions, Violation, run};
use std::fs;
use std::fs::{create_dir_all, write};

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
fn forbidden_pattern_reports_python_source_matches() {
    let root = temp_root("forbidden-python-pattern");
    create_dir_all(root.join("src/service")).expect("create fixture dirs");
    write(
        root.join("klint.yaml"),
        r#"
include: ["src"]
rules: {}
arch:
  forbidden:
    - pattern: "print("
      in: "src/**"
      message: "Use logger"
"#,
    )
    .expect("write config");
    write(
        root.join("src/service/handler.py"),
        "def run():\n    print('debug')\n",
    )
    .expect("write python source");

    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");

    assert_eq!(
        output.violations,
        vec![Violation {
            file: "src/service/handler.py".to_string(),
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
fn forbidden_pattern_reports_swift_source_matches() {
    let root = temp_root("forbidden-swift-pattern");
    create_dir_all(root.join("Sources/App/UI")).expect("create fixture dirs");
    write(
        root.join("klint.yaml"),
        r#"
include: ["Sources"]
rules: {}
arch:
  forbidden:
    - pattern: "URLSession.shared"
      in: "Sources/App/UI/**"
      message: "Use networking client"
"#,
    )
    .expect("write config");
    write(
        root.join("Sources/App/UI/ViewModel.swift"),
        "final class ViewModel {\n    let session = URLSession.shared\n}\n",
    )
    .expect("write swift source");

    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");

    assert_eq!(
        output.violations,
        vec![Violation {
            file: "Sources/App/UI/ViewModel.swift".to_string(),
            line: 2,
            rule: "arch/forbidden".to_string(),
            message: "Use networking client".to_string(),
            severity: "error".to_string(),
            fix: None,
        }]
    );
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
fn singleton_pattern_reports_python_source_matches() {
    let root = temp_root("singleton-python-pattern");
    create_dir_all(root.join("src/lib")).expect("create lib dirs");
    create_dir_all(root.join("src/jobs")).expect("create job dirs");
    write(
        root.join("klint.yaml"),
        r#"
include: ["src"]
rules: {}
arch:
  singleton:
    - pattern: "os.environ[\"API_KEY\"]"
      only: "src/lib/auth.py"
      in: ["src/**"]
      message: "Use auth module"
"#,
    )
    .expect("write config");
    write(
        root.join("src/lib/auth.py"),
        "import os\nKEY = os.environ[\"API_KEY\"]\n",
    )
    .expect("write allowed source");
    write(
        root.join("src/jobs/worker.py"),
        "import os\nKEY = os.environ[\"API_KEY\"]\n",
    )
    .expect("write violating source");

    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");

    assert_eq!(
        output.violations,
        vec![Violation {
            file: "src/jobs/worker.py".to_string(),
            line: 2,
            rule: "arch/singleton".to_string(),
            message: "Use auth module".to_string(),
            severity: "error".to_string(),
            fix: None,
        }]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn singleton_pattern_reports_swift_source_matches() {
    let root = temp_root("singleton-swift-pattern");
    create_dir_all(root.join("Sources/App/Config")).expect("create config dirs");
    create_dir_all(root.join("Sources/App/Jobs")).expect("create job dirs");
    write(
        root.join("klint.yaml"),
        r#"
include: ["Sources"]
rules: {}
arch:
  singleton:
    - pattern: "ProcessInfo.processInfo.environment[\"API_KEY\"]"
      only: "Sources/App/Config/AppConfig.swift"
      in: ["Sources/**"]
      message: "Use AppConfig"
"#,
    )
    .expect("write config");
    write(
        root.join("Sources/App/Config/AppConfig.swift"),
        "let key = ProcessInfo.processInfo.environment[\"API_KEY\"]\n",
    )
    .expect("write allowed source");
    write(
        root.join("Sources/App/Jobs/Worker.swift"),
        "let key = ProcessInfo.processInfo.environment[\"API_KEY\"]\n",
    )
    .expect("write violating source");

    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");

    assert_eq!(
        output.violations,
        vec![Violation {
            file: "Sources/App/Jobs/Worker.swift".to_string(),
            line: 1,
            rule: "arch/singleton".to_string(),
            message: "Use AppConfig".to_string(),
            severity: "error".to_string(),
            fix: None,
        }]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn forbidden_jsx_element_flags_intrinsic_tag_only() {
    let root = temp_root("forbidden-jsx-element");
    create_dir_all(root.join("src/app")).expect("create app dirs");
    write(
        root.join("klint.yaml"),
        r#"
include: ["."]
rules: {}
arch:
  forbidden:
    - jsx-element: button
      in: ["src/app/**/*.tsx"]
      message: "Use Button primitive"
      severity: warn
"#,
    )
    .expect("write config");
    write(
        root.join("src/app/page.tsx"),
        "export default function Page() {\n  return <><Button /><button>Click</button></>;\n}\n",
    )
    .expect("write page");

    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");

    assert_eq!(output.summary.errors, 0);
    assert_eq!(output.summary.warnings, 1);
    assert_eq!(output.violations.len(), 1);
    assert_eq!(output.violations[0].line, 2);
    assert_eq!(output.violations[0].rule, "arch/forbidden");
    assert_eq!(output.violations[0].severity, "warn");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn singleton_jsx_element_ignores_only_file_and_reports_other_matches() {
    let root = temp_root("singleton-jsx-element");
    create_dir_all(root.join("src/components/ui")).expect("create component dirs");
    create_dir_all(root.join("src/app")).expect("create app dirs");
    write(
        root.join("klint.yaml"),
        r#"
include: ["."]
rules: {}
arch:
  singleton:
    - jsx-element: button
      only: src/components/ui/button.tsx
      in: ["src/**/*.tsx"]
      message: "Raw button belongs in the Button primitive"
      severity: warn
"#,
    )
    .expect("write config");
    write(
        root.join("src/components/ui/button.tsx"),
        "export function Button() { return <button />; }\n",
    )
    .expect("write button primitive");
    write(
        root.join("src/app/page.tsx"),
        "export default function Page() { return <button>Click</button>; }\n",
    )
    .expect("write page");

    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");

    assert_eq!(output.summary.errors, 0);
    assert_eq!(output.summary.warnings, 1);
    assert_eq!(output.violations.len(), 1);
    assert_eq!(output.violations[0].file, "src/app/page.tsx");
    assert_eq!(output.violations[0].line, 1);
    assert_eq!(output.violations[0].rule, "arch/singleton");
    let _ = fs::remove_dir_all(root);
}
