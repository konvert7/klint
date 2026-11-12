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
        description = "Deprecated (v1): named rule bundles. plugins: [sonar] is replaced by rules.recommended: true — run `klint migrate` to upgrade the config in place. Available: \"sonar\"."
    )]
    pub(crate) plugins: Option<Vec<String>>,
    #[serde(default)]
    #[schemars(
        schema_with = "crate::schema::rules_schema",
        description = "Rules, in groups: recommended (true enables the six sonar rules at error), core (built-in rules) and sonar (sonar rules, without the sonar/ prefix). Example: { \"recommended\": true, \"core\": { \"no-string-match\": \"error\" }, \"sonar\": { \"prefer-at\": \"warn\" } }. The v1 flat map (rule name → severity or options, sonar/ prefixed) is still read; run `klint migrate` to upgrade it."
    )]
    pub(crate) rules: Option<BTreeMap<String, serde_json::Value>>,
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
pub(crate) enum Severity {
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
        #[schemars(skip)]
        statements: Option<serde_json::Value>,
        #[serde(rename = "allowSetup")]
        #[schemars(skip)]
        allow_setup: Option<serde_json::Value>,
        #[schemars(skip)]
        max: Option<serde_json::Value>,
        #[serde(rename = "skipBlankLines")]
        #[schemars(skip)]
        skip_blank_lines: Option<serde_json::Value>,
    },
}

impl RuleConfig {
    pub(crate) fn statements(&self) -> Option<&serde_json::Value> {
        match self {
            RuleConfig::Severity(_) => None,
            RuleConfig::Options { statements, .. } => statements.as_ref(),
        }
    }

    pub(crate) fn allow_setup(&self) -> Option<&serde_json::Value> {
        match self {
            RuleConfig::Severity(_) => None,
            RuleConfig::Options { allow_setup, .. } => allow_setup.as_ref(),
        }
    }

    pub(crate) fn max(&self) -> Option<&serde_json::Value> {
        match self {
            RuleConfig::Severity(_) => None,
            RuleConfig::Options { max, .. } => max.as_ref(),
        }
    }

    pub(crate) fn skip_blank_lines(&self) -> Option<&serde_json::Value> {
        match self {
            RuleConfig::Severity(_) => None,
            RuleConfig::Options {
                skip_blank_lines, ..
            } => skip_blank_lines.as_ref(),
        }
    }

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

    let yml = config_dir.join("klint.yml");
    if yml.exists() {
        return Ok(yml);
    }

    Err(format!(
        "klint: no config file found — create klint.yaml (or klint.config.json) at {}",
        config_dir.display()
    ))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConfigFormat {
    Yaml,
    Json,
}

impl ConfigFormat {
    pub(crate) fn of(path: &Path) -> ConfigFormat {
        if path.file_name().and_then(|name| name.to_str()) == Some("klint.yaml") {
            ConfigFormat::Yaml
        } else {
            ConfigFormat::Json
        }
    }
}

fn without_byte_order_mark(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}

pub(crate) fn parse_config_text(text: &str, format: ConfigFormat) -> Result<RawConfig, String> {
    let text = without_byte_order_mark(text);
    let raw: RawConfig = match format {
        ConfigFormat::Yaml => serde_yaml::from_str(text).map_err(|err| err.to_string())?,
        ConfigFormat::Json => serde_json::from_str(text).map_err(|err| err.to_string())?,
    };
    crate::config_shape::check_rule_values(&raw)?;
    Ok(raw)
}

pub(crate) fn read_config(path: &Path) -> Result<RawConfig, String> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("klint: failed to read {}: {err}", path.display()))?;
    parse_config_text(&text, ConfigFormat::of(path))
        .map_err(|_| format!("klint: failed to parse {}", path.display()))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn rule_config(yaml: &str) -> RuleConfig {
        serde_yaml::from_str(yaml).expect("valid rule config")
    }

    #[test]
    fn bare_severity_has_no_statements_and_no_allow_setup() {
        let config = rule_config("error");
        assert_eq!(config.statements(), None);
        assert_eq!(config.allow_setup(), None);
    }

    #[test]
    fn statements_list_is_returned_and_allow_setup_is_absent() {
        let config = rule_config("{statements: [if]}");
        assert_eq!(config.statements(), Some(&serde_json::json!(["if"])));
        assert_eq!(config.allow_setup(), None);
    }

    #[test]
    fn max_value_is_returned_and_absent_on_a_bare_severity() {
        assert_eq!(rule_config("{max: 14}").max(), Some(&serde_json::json!(14)));
        assert_eq!(
            rule_config("{max: \"x\"}").max(),
            Some(&serde_json::json!("x"))
        );
        assert_eq!(rule_config("error").max(), None);
        assert_eq!(rule_config("{statements: [if]}").max(), None);
    }

    #[test]
    fn allow_setup_false_is_returned_and_statements_is_absent() {
        let config = rule_config("{allowSetup: false}");
        assert_eq!(config.statements(), None);
        assert_eq!(config.allow_setup(), Some(&serde_json::json!(false)));
    }

    #[test]
    fn bare_severity_has_no_skip_blank_lines() {
        assert_eq!(rule_config("error").skip_blank_lines(), None);
    }

    #[test]
    fn boolean_skip_blank_lines_is_returned() {
        assert_eq!(
            rule_config("{skipBlankLines: true}").skip_blank_lines(),
            Some(&serde_json::json!(true))
        );
    }

    #[test]
    fn string_skip_blank_lines_is_returned_unchanged() {
        assert_eq!(
            rule_config("{skipBlankLines: \"yes\"}").skip_blank_lines(),
            Some(&serde_json::json!("yes"))
        );
    }

    #[test]
    fn options_without_skip_blank_lines_return_none() {
        assert_eq!(rule_config("{max: 3}").skip_blank_lines(), None);
    }
}
