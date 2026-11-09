use std::collections::BTreeMap;

use crate::config::RuleConfig;

const SONAR_RULES: &[&str] = &[
    "sonar/prefer-string-replaceall",
    "sonar/prefer-string-raw-regexp",
    "sonar/prefer-string-raw",
    "sonar/prefer-nullish-coalescing-assign",
    "sonar/no-single-char-class",
    "sonar/prefer-at",
];

fn plugin_rules(plugin: &str) -> Option<&'static [&'static str]> {
    match plugin {
        "sonar" => Some(SONAR_RULES),
        _ => None,
    }
}

pub(crate) fn effective_rules(
    plugins: &[String],
    explicit: BTreeMap<String, RuleConfig>,
) -> Result<BTreeMap<String, RuleConfig>, String> {
    let mut rules = BTreeMap::new();
    for plugin in plugins {
        let defaults = plugin_rules(plugin)
            .ok_or_else(|| format!("klint: Unknown klint plugin: \"{plugin}\""))?;
        for rule in defaults {
            rules.insert(
                (*rule).to_string(),
                RuleConfig::Severity("error".to_string()),
            );
        }
    }
    rules.extend(explicit);
    Ok(rules)
}
