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
    arch: Option<ArchConfig>,
}

#[derive(Debug, Deserialize)]
struct ArchConfig {
    layers: Option<BTreeMap<String, Vec<String>>>,
    forbidden: Option<Vec<ArchForbiddenRule>>,
}

#[derive(Debug, Deserialize)]
struct ArchForbiddenRule {
    pattern: Option<String>,
    #[serde(rename = "jsx-element")]
    jsx_element: Option<serde_yaml::Value>,
    #[serde(rename = "in")]
    in_scope: StringOrVec,
    message: String,
    severity: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StringOrVec {
    One(String),
    Many(Vec<String>),
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

    let files = resolve_files(&root, &include)?;
    let file_contents = read_files(&files)?;
    let _plugins = raw.plugins.unwrap_or_default();
    let _rules = raw.rules.unwrap_or_default();

    let mut violations = Vec::new();
    if let Some(arch) = raw.arch {
        run_arch_forbidden_rules(&arch, &files, &file_contents, &root, &mut violations);
    }

    Ok(output_from_violations(violations))
}

pub fn empty_output() -> JsonOutput {
    output_from_violations(Vec::new())
}

fn output_from_violations(violations: Vec<Violation>) -> JsonOutput {
    let summary = Summary {
        errors: violations
            .iter()
            .filter(|violation| violation.severity == "error")
            .count(),
        warnings: violations
            .iter()
            .filter(|violation| violation.severity == "warn")
            .count(),
    };

    JsonOutput {
        violations,
        summary,
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

fn read_files(files: &[PathBuf]) -> Result<BTreeMap<PathBuf, String>, String> {
    let mut contents = BTreeMap::new();
    for file in files {
        let content = fs::read_to_string(file)
            .map_err(|err| format!("klint-rs: failed to read {}: {err}", file.display()))?;
        contents.insert(file.clone(), content);
    }
    Ok(contents)
}

fn run_arch_forbidden_rules(
    arch: &ArchConfig,
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    root: &Path,
    violations: &mut Vec<Violation>,
) {
    let Some(rules) = &arch.forbidden else {
        return;
    };

    for rule in rules {
        let Some(pattern) = &rule.pattern else {
            continue;
        };
        if rule.jsx_element.is_some() {
            continue;
        }

        let scoped_files = resolve_layer_files(&rule.in_scope, arch.layers.as_ref(), root, files);
        scan_lines_for_pattern(
            &scoped_files,
            file_contents,
            root,
            pattern,
            &rule.message,
            rule.severity.as_deref().unwrap_or("error"),
            violations,
        );
    }
}

fn resolve_layer_files(
    scope: &StringOrVec,
    layers: Option<&BTreeMap<String, Vec<String>>>,
    root: &Path,
    all_files: &[PathBuf],
) -> Vec<PathBuf> {
    let globs = resolve_globs(scope, layers);
    let include_prefixes: Vec<PathBuf> = globs
        .iter()
        .filter(|glob| !glob.starts_with('!'))
        .map(|glob| glob_to_prefix(glob, root))
        .collect();
    let exclude_prefixes: Vec<PathBuf> = globs
        .iter()
        .filter(|glob| glob.starts_with('!'))
        .map(|glob| glob_to_prefix(&glob[1..], root))
        .collect();

    all_files
        .iter()
        .filter(|file| {
            include_prefixes
                .iter()
                .any(|prefix| path_in_prefix(file, prefix))
                && !exclude_prefixes
                    .iter()
                    .any(|prefix| path_in_prefix(file, prefix))
        })
        .cloned()
        .collect()
}

fn resolve_globs(
    scope: &StringOrVec,
    layers: Option<&BTreeMap<String, Vec<String>>>,
) -> Vec<String> {
    scope
        .items()
        .iter()
        .flat_map(|item| {
            layers
                .and_then(|known| known.get(item))
                .cloned()
                .unwrap_or_else(|| vec![item.clone()])
        })
        .collect()
}

impl StringOrVec {
    fn items(&self) -> Vec<String> {
        match self {
            Self::One(item) => vec![item.clone()],
            Self::Many(items) => items.clone(),
        }
    }
}

fn glob_to_prefix(glob: &str, root: &Path) -> PathBuf {
    let prefix = glob
        .split("/**")
        .next()
        .unwrap_or(glob)
        .split("/*")
        .next()
        .unwrap_or(glob)
        .split('*')
        .next()
        .unwrap_or(glob);
    normalize_path(&root.join(prefix))
}

fn path_in_prefix(path: &Path, prefix: &Path) -> bool {
    path == prefix || path.starts_with(prefix)
}

fn scan_lines_for_pattern(
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    root: &Path,
    pattern: &str,
    message: &str,
    severity: &str,
    violations: &mut Vec<Violation>,
) {
    for file in files {
        let Some(content) = file_contents.get(file) else {
            continue;
        };

        for (index, line) in content.lines().enumerate() {
            if line.contains(pattern) {
                violations.push(Violation {
                    file: relative_path(root, file),
                    line: index + 1,
                    rule: "arch/forbidden".to_string(),
                    message: message.to_string(),
                    severity: severity.to_string(),
                    fix: None,
                });
            }
        }
    }
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

fn relative_path(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .to_string_lossy()
        .replace('\\', "/")
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
}
