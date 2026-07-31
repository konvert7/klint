mod common;

use common::temp_root;
use klint_rs::{RunOptions, Violation, empty_output, run};
use std::fs;
use std::fs::{create_dir_all, write};

#[test]
fn imports_deny_mode_flags_swift_module_imports() {
    let root = temp_root("imports-swift-module");
    create_dir_all(root.join("Sources/App/UI")).expect("create ui dirs");
    create_dir_all(root.join("Sources/App/Core")).expect("create core dirs");
    write(
        root.join("klint.yaml"),
        r#"
include: ["Sources"]
rules: {}
arch:
  layers:
    ui: ["Sources/App/UI/**"]
    core: ["Sources/App/Core/**"]
  imports:
    - from: ui
      deny: core
      message: "UI must not import core directly"
"#,
    )
    .expect("write config");
    write(
        root.join("Sources/App/UI/ViewModel.swift"),
        "import Foundation\nimport Core\n",
    )
    .expect("write swift source");
    write(root.join("Sources/App/Core/Auth.swift"), "struct Auth {}\n").expect("write core source");

    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");

    assert_eq!(
        output.violations,
        vec![Violation {
            file: "Sources/App/UI/ViewModel.swift".to_string(),
            line: 2,
            rule: "arch/imports".to_string(),
            message: "UI must not import core directly".to_string(),
            severity: "error".to_string(),
            fix: None,
        }]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn imports_allow_mode_accepts_swift_modules_in_allowlist() {
    let root = temp_root("imports-swift-allow");
    create_dir_all(root.join("Sources/App/UI")).expect("create ui dirs");
    create_dir_all(root.join("Sources/App/DesignSystem")).expect("create design dirs");
    write(
        root.join("klint.yaml"),
        r#"
include: ["Sources"]
rules: {}
arch:
  layers:
    ui: ["Sources/App/UI/**"]
    design: ["Sources/App/DesignSystem/**"]
  imports:
    - from: ui
      allow: [ui, design]
      message: "UI may only import UI and DesignSystem"
"#,
    )
    .expect("write config");
    write(
        root.join("Sources/App/UI/View.swift"),
        "import Foundation\nimport DesignSystem\n",
    )
    .expect("write swift source");
    write(
        root.join("Sources/App/DesignSystem/Button.swift"),
        "public struct Button {}\n",
    )
    .expect("write design source");

    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");

    assert_eq!(output, empty_output());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn imports_deny_mode_flags_csharp_namespace_across_layers() {
    let root = temp_root("imports-csharp-deny");
    create_dir_all(root.join("src/Web")).expect("create web dirs");
    create_dir_all(root.join("src/Data")).expect("create data dirs");
    write(
        root.join("klint.yaml"),
        r#"
include: ["src"]
rules: {}
arch:
  layers:
    web: ["src/Web/**"]
    data: ["src/Data/**"]
  imports:
    - from: web
      deny: data
      message: "Web must not import data directly"
"#,
    )
    .expect("write config");
    write(
        root.join("src/Web/HomeController.cs"),
        "using System;\nusing App.Data;\n",
    )
    .expect("write web source");
    write(
        root.join("src/Data/Repository.cs"),
        "namespace App.Data;\nclass Repository {}\n",
    )
    .expect("write data source");

    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");

    assert_eq!(
        output.violations,
        vec![Violation {
            file: "src/Web/HomeController.cs".to_string(),
            line: 2,
            rule: "arch/imports".to_string(),
            message: "Web must not import data directly".to_string(),
            severity: "error".to_string(),
            fix: None,
        }]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn imports_allow_mode_flags_csharp_namespace_outside_allowlist() {
    // Non-vacuous: an unresolved `using` would produce no violation, so a broken
    // namespace resolver turns this test red rather than passing silently.
    let root = temp_root("imports-csharp-allow");
    create_dir_all(root.join("src/Web")).expect("create web dirs");
    create_dir_all(root.join("src/Shared")).expect("create shared dirs");
    write(
        root.join("klint.yaml"),
        r#"
include: ["src"]
rules: {}
arch:
  layers:
    web: ["src/Web/**"]
    shared: ["src/Shared/**"]
  imports:
    - from: web
      allow: [web]
      message: "Web may only import Web"
"#,
    )
    .expect("write config");
    write(
        root.join("src/Web/HomeController.cs"),
        "using App.Shared;\n",
    )
    .expect("write web source");
    write(
        root.join("src/Shared/Logger.cs"),
        "namespace App.Shared;\nclass Logger {}\n",
    )
    .expect("write shared source");

    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");

    assert_eq!(
        output.violations,
        vec![Violation {
            file: "src/Web/HomeController.cs".to_string(),
            line: 1,
            rule: "arch/imports".to_string(),
            message: "Web may only import Web".to_string(),
            severity: "error".to_string(),
            fix: None,
        }]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn deny_packages_flags_csharp_namespace_imports() {
    let root = temp_root("deny-packages-csharp");
    create_dir_all(root.join("src/Web")).expect("create web dirs");
    write(
        root.join("klint.yaml"),
        r#"
include: ["src"]
rules: {}
arch:
  imports:
    - from: src/**
      deny-packages: [Newtonsoft]
      message: "Use System.Text.Json instead of Newtonsoft"
"#,
    )
    .expect("write config");
    write(
        root.join("src/Web/HomeController.cs"),
        "using System;\nusing Newtonsoft.Json;\n",
    )
    .expect("write csharp source");

    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");

    assert_eq!(
        output.violations,
        vec![Violation {
            file: "src/Web/HomeController.cs".to_string(),
            line: 2,
            rule: "arch/imports".to_string(),
            message: "Use System.Text.Json instead of Newtonsoft".to_string(),
            severity: "error".to_string(),
            fix: None,
        }]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn imports_deny_mode_flags_python_relative_imports() {
    let root = temp_root("imports-python-relative");
    create_dir_all(root.join("src/jobs")).expect("create jobs dirs");
    create_dir_all(root.join("src/lib")).expect("create lib dirs");
    write(
        root.join("klint.yaml"),
        r#"
include: ["src"]
rules: {}
arch:
  layers:
    jobs: ["src/jobs/**"]
    lib: ["src/lib/**"]
  imports:
    - from: jobs
      deny: lib
      message: "Jobs must not import lib directly"
"#,
    )
    .expect("write config");
    write(
        root.join("src/jobs/worker.py"),
        "import requests\nfrom ..lib.auth import load_key\n",
    )
    .expect("write importing source");
    write(
        root.join("src/lib/auth.py"),
        "def load_key():\n    return 'x'\n",
    )
    .expect("write lib source");

    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");

    assert_eq!(
        output.violations,
        vec![Violation {
            file: "src/jobs/worker.py".to_string(),
            line: 2,
            rule: "arch/imports".to_string(),
            message: "Jobs must not import lib directly".to_string(),
            severity: "error".to_string(),
            fix: None,
        }]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn imports_deny_mode_flags_python_absolute_imports_under_source_root() {
    let root = temp_root("imports-python-absolute-src");
    create_dir_all(root.join("src/app/jobs")).expect("create jobs dirs");
    create_dir_all(root.join("src/app/lib")).expect("create lib dirs");
    write(
        root.join("klint.yaml"),
        r#"
include: ["src"]
rules: {}
arch:
  layers:
    jobs: ["src/app/jobs/**"]
    lib: ["src/app/lib/**"]
  imports:
    - from: jobs
      deny: lib
      message: "Jobs must not import lib directly"
"#,
    )
    .expect("write config");
    write(
        root.join("src/app/jobs/worker.py"),
        "from app.lib.auth import load_key\n",
    )
    .expect("write importing source");
    write(
        root.join("src/app/lib/auth.py"),
        "def load_key():\n    return 'x'\n",
    )
    .expect("write lib source");

    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");

    assert_eq!(
        output.violations,
        vec![Violation {
            file: "src/app/jobs/worker.py".to_string(),
            line: 1,
            rule: "arch/imports".to_string(),
            message: "Jobs must not import lib directly".to_string(),
            severity: "error".to_string(),
            fix: None,
        }]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn imports_deny_mode_flags_python_absolute_imports_under_project_root() {
    let root = temp_root("imports-python-absolute-root");
    create_dir_all(root.join("app/jobs")).expect("create jobs dirs");
    create_dir_all(root.join("app/lib")).expect("create lib dirs");
    write(
        root.join("klint.yaml"),
        r#"
include: ["."]
rules: {}
arch:
  layers:
    jobs: ["app/jobs/**"]
    lib: ["app/lib/**"]
  imports:
    - from: jobs
      deny: lib
      message: "Jobs must not import lib directly"
"#,
    )
    .expect("write config");
    write(root.join("app/jobs/worker.py"), "import app.lib.auth\n")
        .expect("write importing source");
    write(
        root.join("app/lib/auth.py"),
        "def load_key():\n    return 'x'\n",
    )
    .expect("write lib source");

    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");

    assert_eq!(output.summary.errors, 1);
    assert_eq!(output.violations[0].line, 1);
    assert_eq!(output.violations[0].file, "app/jobs/worker.py");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn imports_deny_mode_flags_python_package_init_imports() {
    let root = temp_root("imports-python-package-init");
    create_dir_all(root.join("src/app/jobs")).expect("create jobs dirs");
    create_dir_all(root.join("src/app/lib/auth")).expect("create lib package dirs");
    write(
        root.join("klint.yaml"),
        r#"
include: ["src"]
rules: {}
arch:
  layers:
    jobs: ["src/app/jobs/**"]
    lib: ["src/app/lib/**"]
  imports:
    - from: jobs
      deny: lib
      message: "Jobs must not import lib directly"
"#,
    )
    .expect("write config");
    write(
        root.join("src/app/jobs/worker.py"),
        "from app.lib.auth import load_key\n",
    )
    .expect("write importing source");
    write(
        root.join("src/app/lib/auth/__init__.py"),
        "def load_key():\n    return 'x'\n",
    )
    .expect("write lib package");

    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");

    assert_eq!(output.summary.errors, 1);
    assert_eq!(output.violations[0].file, "src/app/jobs/worker.py");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn imports_deny_mode_ignores_unresolved_python_packages() {
    let root = temp_root("imports-python-package");
    create_dir_all(root.join("src/jobs")).expect("create jobs dirs");
    create_dir_all(root.join("src/lib")).expect("create lib dirs");
    write(
        root.join("klint.yaml"),
        r#"
include: ["src"]
rules: {}
arch:
  layers:
    jobs: ["src/jobs/**"]
    lib: ["src/lib/**"]
  imports:
    - from: jobs
      deny: lib
      message: "Jobs must not import lib directly"
"#,
    )
    .expect("write config");
    write(root.join("src/jobs/worker.py"), "import requests\n").expect("write importing source");
    write(
        root.join("src/lib/auth.py"),
        "def load_key():\n    return 'x'\n",
    )
    .expect("write lib source");

    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");

    assert_eq!(output, empty_output());
    let _ = fs::remove_dir_all(root);
}
