mod common;

use common::temp_root;
use klint_rs::{RunOptions, run};
use std::fs;
use std::fs::{create_dir_all, write};

fn violation_lines(name: &str, ext: &str) -> Vec<usize> {
    let root = temp_root(name);
    create_dir_all(root.join("scripts/ui")).expect("create ui dirs");
    create_dir_all(root.join("scripts/core")).expect("create core dirs");
    write(
        root.join("klint.yaml"),
        r#"
include: ["."]
rules: {}
arch:
  layers:
    ui: ["scripts/ui/**"]
    core: ["scripts/core/**"]
  imports:
    - from: ui
      deny: core
      message: "UI must not import core"
"#,
    )
    .expect("write config");
    write(
        root.join(format!("scripts/ui/main.{ext}")),
        format!("import x from \"../core/util.{ext}\";\nexport default x;\n"),
    )
    .expect("write importing source");
    write(
        root.join(format!("scripts/core/util.{ext}")),
        "export default 1;\n",
    )
    .expect("write imported source");
    let output = run(RunOptions {
        config_dir: root.clone(),
    })
    .expect("valid config should run");
    let _ = fs::remove_dir_all(root);
    output.violations.iter().map(|v| v.line).collect()
}

#[test]
fn imports_deny_mode_flags_mjs_import_with_explicit_extension() {
    assert_eq!(violation_lines("imports-mjs", "mjs"), vec![1]);
}

#[test]
fn imports_deny_mode_flags_cjs_import_with_explicit_extension() {
    assert_eq!(violation_lines("imports-cjs", "cjs"), vec![1]);
}
