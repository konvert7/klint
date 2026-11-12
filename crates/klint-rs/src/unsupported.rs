use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::config::RuleConfig;
use crate::config_shape::ResolvedRules;
use crate::migrate::v2_location;
use crate::output::Violation;
use crate::rules::is_supported_rule;

const RULE: &str = "klint/unsupported-rule";
const CUSTOM_RULES_FILE: &str = "klint.rules.ts";

const TYPE_CHECKER_RULES: &[(&str, Option<&str>)] = &[
    ("no-floating-promise", Some("nursery/noFloatingPromises")),
    ("no-misused-promises", Some("nursery/noMisusedPromises")),
    ("no-object-in-template", Some("nursery/noBaseToString")),
    ("no-date-equality", None),
    ("no-optional-chain-on-non-nullable", None),
];

pub(crate) fn is_type_checker_rule(name: &str) -> bool {
    TYPE_CHECKER_RULES.iter().any(|(rule, _)| *rule == name)
}

pub(crate) fn unsupported_rule_advisories(
    rules: &BTreeMap<String, RuleConfig>,
    resolved: &ResolvedRules,
    config_path: &Path,
    config_dir: &Path,
) -> Vec<Violation> {
    let config_text = fs::read_to_string(config_path).unwrap_or_default();
    let config_file = config_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("klint.yaml");

    let mut advisories: Vec<Violation> = rules
        .iter()
        .filter(|(name, config)| config.severity() != "off" && !is_supported_rule(name))
        .map(|(name, _)| {
            advisory(
                config_file,
                line_naming(&config_text, resolved.written_key(name)),
                unsupported_message(name),
            )
        })
        .collect();

    for written in &resolved.misplaced {
        let (group, short) = v2_location(written);
        advisories.push(advisory(
            config_file,
            line_naming(&config_text, written),
            format!(
                "\"{written}\" is now rules.{}.{short} — run `klint migrate`.",
                group.key()
            ),
        ));
    }

    if config_dir.join(CUSTOM_RULES_FILE).exists() {
        advisories.push(advisory(
            CUSTOM_RULES_FILE,
            1,
            format!(
                "Custom rules in {CUSTOM_RULES_FILE} are not supported by klint's Rust engine and are ignored."
            ),
        ));
    }
    advisories
}

fn unsupported_message(rule: &str) -> String {
    match TYPE_CHECKER_RULES.iter().find(|(name, _)| *name == rule) {
        Some((_, Some(biome_rule))) => format!(
            "{rule} needs a TypeScript type checker, which klint's Rust engine does not have, so it is ignored — Biome's {biome_rule} covers it."
        ),
        Some((_, None)) => format!(
            "{rule} needs a TypeScript type checker, which klint's Rust engine does not have, so it is ignored."
        ),
        None => format!("Unknown rule \"{rule}\" is ignored — check its name."),
    }
}

fn line_naming(config_text: &str, rule: &str) -> usize {
    config_text
        .lines()
        .position(|line| line.contains(rule))
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
