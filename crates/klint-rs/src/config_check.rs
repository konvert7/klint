use std::fs;
use std::path::Path;

use crate::config::RuleConfig;
use crate::config_shape::{ConfigShape, ResolvedRules, shape_advisories};
use crate::output::Violation;
use crate::rules::{
    effective_severity, old_form_message, rule_option_problems, uses_old_threshold_form,
};

const RULE: &str = "klint/config";
const KNOWN_KEYS: &[&str] = &["$schema", "root", "include", "plugins", "rules", "arch"];
const SEVERITIES: &[&str] = &["error", "warn", "off"];

pub(crate) fn has_valid_severity(name: &str, config: &RuleConfig) -> bool {
    SEVERITIES.contains(&effective_severity(name, config))
}

pub(crate) fn config_advisories(resolved: &ResolvedRules, config_path: &Path) -> Vec<Violation> {
    let text = fs::read_to_string(config_path).unwrap_or_default();
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let file = config_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("klint.yaml");

    let unknown_keys = top_level_keys(text)
        .into_iter()
        .filter(|key| !KNOWN_KEYS.contains(&key.as_str()))
        .map(|key| {
            advisory(
                file,
                line_of_key(text, &key),
                format!("Unknown config key \"{key}\" is ignored — check its name."),
            )
        });
    unknown_keys
        .chain(invalid_severity_advisories(resolved, text, file))
        .chain(option_problem_advisories(resolved, text, file))
        .chain(old_form_advisories(resolved, text, file))
        .chain(shape_advisories(resolved, text, file))
        .collect()
}

fn invalid_severity_advisories(resolved: &ResolvedRules, text: &str, file: &str) -> Vec<Violation> {
    resolved
        .rules
        .iter()
        .filter(|(name, config)| !has_valid_severity(name, config))
        .map(|(name, config)| {
            advisory(
                file,
                line_of_key(text, resolved.written_key(name)),
                format!(
                    "Rule \"{name}\" has severity \"{}\", which is not error, warn or off, so it is ignored.",
                    effective_severity(name, config)
                ),
            )
        })
        .collect()
}

fn option_problem_advisories(resolved: &ResolvedRules, text: &str, file: &str) -> Vec<Violation> {
    resolved
        .rules
        .iter()
        .flat_map(|(name, config)| {
            rule_option_problems(name, config)
                .into_iter()
                .map(move |problem| {
                    advisory(file, line_of_key(text, resolved.written_key(name)), problem)
                })
        })
        .collect()
}

fn old_form_advisories(resolved: &ResolvedRules, text: &str, file: &str) -> Vec<Violation> {
    if resolved.shape == ConfigShape::V1 {
        return Vec::new();
    }

    resolved
        .rules
        .iter()
        .filter(|(name, config)| uses_old_threshold_form(name, config))
        .map(|(name, _)| {
            advisory(
                file,
                line_of_key(text, resolved.written_key(name)),
                old_form_message(name),
            )
        })
        .collect()
}

fn top_level_keys(text: &str) -> Vec<String> {
    match serde_yaml::from_str::<serde_yaml::Value>(text) {
        Ok(serde_yaml::Value::Mapping(map)) => map
            .keys()
            .filter_map(|key| key.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

pub(crate) fn line_of_key(text: &str, key: &str) -> usize {
    let declares = |line: &str| {
        let line = line.trim_start();
        [format!("{key}:"), format!("\"{key}\""), format!("'{key}'")]
            .iter()
            .any(|form| line.starts_with(form.as_str()))
    };
    text.lines()
        .position(declares)
        .or_else(|| text.lines().position(|line| line.contains(key)))
        .map_or(1, |index| index + 1)
}

fn advisory(file: &str, line: usize, message: String) -> Violation {
    Violation {
        file: file.to_string(),
        line,
        rule: RULE.to_string(),
        message,
        severity: "warn".to_string(),
        fix: None,
    }
}
