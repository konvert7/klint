mod common;

use common::temp_root;
use klint_rs::{RunOptions, empty_output, run};
use std::fs;
use std::fs::{create_dir_all, write};

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
    write(root.join("klint.yaml"), "include: [\"src\"]\nrules: {}\n").expect("write yaml config");
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
