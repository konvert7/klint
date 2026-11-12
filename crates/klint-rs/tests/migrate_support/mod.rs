#![allow(dead_code)]

use crate::common::temp_root;
use serde_json::Value;
use std::fs::{create_dir_all, remove_dir_all, write};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const SOURCE: &str = "const a = arr[arr.length - 1];\nconst hit = \"abc\".match(/a/);\n";

pub struct Project {
    pub root: PathBuf,
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = remove_dir_all(&self.root);
    }
}

pub struct Ran {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Ran {
    pub fn all(&self) -> String {
        format!("{}\n{}", self.stdout, self.stderr)
    }
}

pub fn project(config_file: &str, config: &[u8]) -> Project {
    let root = temp_root("migrate");
    create_dir_all(root.join("src")).expect("create fixture dirs");
    write(root.join(config_file), config).expect("write config");
    write(root.join("src/a.ts"), SOURCE).expect("write source");
    Project { root }
}

pub fn klint(root: &Path, args: &[&str]) -> Ran {
    let output = Command::new(env!("CARGO_BIN_EXE_klint-rs"))
        .args(args)
        .args(["--config", &root.to_string_lossy()])
        .env("KLINT_NO_STATS", "1")
        .output()
        .expect("klint-rs runs");
    Ran {
        status: output.status.code().expect("exits normally"),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

pub fn lint_json(root: &Path) -> Value {
    let ran = klint(root, &["--json"]);
    serde_json::from_str(&ran.stdout).unwrap_or_else(|e| panic!("json: {e}: {}", ran.all()))
}

pub fn violations(json: &Value) -> Vec<Value> {
    json["violations"].as_array().expect("violations").clone()
}

pub fn without_config_warnings(json: &Value) -> Vec<Value> {
    violations(json)
        .into_iter()
        .filter(|v| v["rule"] != "klint/config")
        .collect()
}

pub fn config_warnings(json: &Value) -> Vec<Value> {
    violations(json)
        .into_iter()
        .filter(|v| v["rule"] == "klint/config")
        .collect()
}

pub fn names_line(text: &str, line: usize) -> bool {
    text.contains(&format!("line {line}")) || text.contains(&format!(":{line}"))
}
