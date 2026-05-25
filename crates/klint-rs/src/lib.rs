use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct RawConfig {
    root: Option<String>,
    include: Option<Vec<String>>,
    plugins: Option<Vec<String>>,
    rules: Option<BTreeMap<String, serde_yaml::Value>>,
    arch: Option<serde_yaml::Value>,
}

#[derive(Debug)]
pub struct RunOptions {
    pub config_dir: PathBuf,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Violation {
    pub file: String,
    pub line: usize,
    pub rule: String,
    pub message: String,
    pub severity: String,
    pub fix: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Summary {
    pub errors: usize,
    pub warnings: usize,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct JsonOutput {
    pub violations: Vec<Violation>,
    pub summary: Summary,
}

pub fn run(options: RunOptions) -> Result<JsonOutput, String> {
    let config_path = find_config(&options.config_dir)?;
    let raw = read_config(&config_path)?;
    let root = resolve_root(&options.config_dir, raw.root.as_deref());
    let include = raw.include.unwrap_or_else(|| vec![".".to_string()]);

    let _files = resolve_files(&root, &include)?;
    let _plugins = raw.plugins.unwrap_or_default();
    let _rules = raw.rules.unwrap_or_default();
    let _arch = raw.arch;

    Ok(empty_output())
}

pub fn empty_output() -> JsonOutput {
    JsonOutput {
        violations: Vec::new(),
        summary: Summary {
            errors: 0,
            warnings: 0,
        },
    }
}

fn find_config(config_dir: &Path) -> Result<PathBuf, String> {
    let yaml = config_dir.join("klint.yaml");
    if yaml.exists() {
        return Ok(yaml);
    }

    let json = config_dir.join("klint.config.json");
    if json.exists() {
        return Ok(json);
    }

    Err(format!(
        "klint-rs: no config file found — create klint.yaml (or klint.config.json) at {}",
        config_dir.display()
    ))
}

fn read_config(path: &Path) -> Result<RawConfig, String> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("klint-rs: failed to read {}: {err}", path.display()))?;

    if path.file_name().and_then(|name| name.to_str()) == Some("klint.yaml") {
        serde_yaml::from_str(&text)
            .map_err(|_| format!("klint-rs: failed to parse {}", path.display()))
    } else {
        serde_json::from_str(&text)
            .map_err(|_| format!("klint-rs: failed to parse {}", path.display()))
    }
}

fn resolve_root(config_dir: &Path, root: Option<&str>) -> PathBuf {
    let configured = root.unwrap_or(".");
    let path = Path::new(configured);
    if path.is_absolute() {
        normalize_path(path)
    } else {
        normalize_path(&config_dir.join(path))
    }
}

fn resolve_files(root: &Path, include: &[String]) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for pattern in include.iter().filter(|pattern| !pattern.starts_with('!')) {
        let base = include_base(root, pattern);
        collect_ts_files(&base, &mut files)?;
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn include_base(root: &Path, pattern: &str) -> PathBuf {
    let prefix = pattern
        .split("/**")
        .next()
        .unwrap_or(pattern)
        .split("/*")
        .next()
        .unwrap_or(pattern);
    normalize_path(&root.join(prefix))
}

fn collect_ts_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Ok(());
    };

    for entry in entries {
        let entry = entry.map_err(|err| format!("klint-rs: failed to read dir entry: {err}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_ts_files(&path, files)?;
        } else if is_supported_source(&path) {
            files.push(normalize_path(&path));
        }
    }
    Ok(())
}

fn is_supported_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("ts" | "tsx" | "js" | "jsx" | "mts" | "cts")
    )
}

fn normalize_path(path: &Path) -> PathBuf {
    let text = path.to_string_lossy().replace('\\', "/");
    PathBuf::from(text)
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
