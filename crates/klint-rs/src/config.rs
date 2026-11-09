use crate::arch::ArchConfig;
use crate::files::normalize_path;
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(
    deny_unknown_fields,
    description = "klint configuration. Lives at klint.yaml (or klint.config.json) next to biome.json and knip.json."
)]
pub(crate) struct RawConfig {
    #[serde(rename = "$schema")]
    #[schemars(
        description = "JSON Schema reference. Use ./klint.schema.json for local validation, or a version-tagged URL such as https://raw.githubusercontent.com/konvert7/klint/refs/tags/v0.29.0/klint.schema.json to pin the schema. A tagged URL naming a version other than the installed one raises a klint/schema-version warning."
    )]
    pub(crate) schema: Option<String>,
    #[schemars(
        description = "Root directory used to resolve include paths and report relative file names. Defaults to the directory containing klint.config.json."
    )]
    pub(crate) root: Option<String>,
    #[schemars(
        description = "Glob patterns selecting which TypeScript files to lint. Prefix with ! to exclude. Defaults to [\".\"] which lints all .ts files under root. Example: [\"src\", \"klint\", \"!**/node_modules/**\"]"
    )]
    pub(crate) include: Option<Vec<String>>,
    #[schemars(
        with = "Option<Vec<PluginName>>",
        description = "Named rule bundles to enable. Each plugin applies a default set of rules at \"error\" severity. Individual rules from the bundle can be overridden or silenced via the rules map. Available: \"sonar\"."
    )]
    pub(crate) plugins: Option<Vec<String>>,
    #[serde(default)]
    #[schemars(
        schema_with = "crate::schema::rules_schema",
        description = "Map of rule name → severity or options. Example: { \"no-floating-promise\": \"error\", \"no-sync-in-async\": { \"severity\": \"warn\", \"include\": [\"src/hooks/**\"] } }."
    )]
    pub(crate) rules: Option<BTreeMap<String, RuleConfig>>,
    pub(crate) arch: Option<ArchConfig>,
}

#[derive(JsonSchema)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
enum PluginName {
    Sonar,
}

#[derive(JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(
    description = "Rule severity. \"error\" exits with code 2; \"warn\" reports but exits 0; \"off\" silences."
)]
#[allow(dead_code)]
enum Severity {
    Error,
    Warn,
    Off,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(untagged)]
pub(crate) enum RuleConfig {
    Severity(#[schemars(with = "Severity")] String),
    #[schemars(
        extend("additionalProperties" = false),
        description = "Rule options object. Omit severity to default to \"error\". Add include to scope the rule to specific files."
    )]
    Options {
        #[schemars(with = "Option<Severity>")]
        severity: Option<String>,
        #[schemars(
            description = "Glob patterns scoping this rule to a subset of files. Prefix with ! to exclude. Example: [\"src/hooks/**\", \"!src/hooks/scripts/**\"]"
        )]
        include: Option<Vec<String>>,
    },
}

impl RuleConfig {
    pub(crate) fn severity(&self) -> &str {
        match self {
            RuleConfig::Severity(value) => value,
            RuleConfig::Options { severity, .. } => severity.as_deref().unwrap_or("error"),
        }
    }

    pub(crate) fn include(&self) -> Option<&[String]> {
        match self {
            RuleConfig::Severity(_) => None,
            RuleConfig::Options { include, .. } => include.as_deref(),
        }
    }
}

pub(crate) fn find_config(config_dir: &Path) -> Result<PathBuf, String> {
    let yaml = config_dir.join("klint.yaml");
    if yaml.exists() {
        return Ok(yaml);
    }

    let json = config_dir.join("klint.config.json");
    if json.exists() {
        return Ok(json);
    }

    Err(format!(
        "klint: no config file found — create klint.yaml (or klint.config.json) at {}",
        config_dir.display()
    ))
}

pub(crate) fn read_config(path: &Path) -> Result<RawConfig, String> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("klint: failed to read {}: {err}", path.display()))?;

    if path.file_name().and_then(|name| name.to_str()) == Some("klint.yaml") {
        serde_yaml::from_str(&text)
            .map_err(|_| format!("klint: failed to parse {}", path.display()))
    } else {
        serde_json::from_str(&text)
            .map_err(|_| format!("klint: failed to parse {}", path.display()))
    }
}

pub(crate) fn resolve_root(config_dir: &Path, root: Option<&str>) -> PathBuf {
    let configured = root.unwrap_or(".");
    let path = Path::new(configured);
    if path.is_absolute() {
        normalize_path(path)
    } else {
        normalize_path(&config_dir.join(path))
    }
}
