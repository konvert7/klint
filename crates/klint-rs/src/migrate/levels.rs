use serde_json::Value;

use crate::config::{RawConfig, RuleConfig};
use crate::config_check::line_of_key;
use crate::config_shape::resolve_rules;
use crate::rules::{Level, ThresholdSpec, threshold_spec, uses_old_threshold_form};

pub(super) struct LevelPlan {
    pub(super) rule: &'static str,
    pub(super) change: LevelChange,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LevelChange {
    RenameMax { to: Level },
    ReplaceSeverity { to: Level, limit: u32 },
    TurnOff,
    InsertLevel { limit: u32 },
}

pub(super) fn needs_levels(raw: &RawConfig) -> bool {
    resolve_rules(raw).is_ok_and(|resolved| {
        resolved
            .rules
            .iter()
            .any(|(id, config)| uses_old_threshold_form(id, config))
    })
}

pub(super) fn level_plans(raw: &RawConfig, text: &str) -> Result<Vec<LevelPlan>, String> {
    let Ok(resolved) = resolve_rules(raw) else {
        return Ok(Vec::new());
    };
    let body = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut plans = Vec::new();

    for (id, config) in &resolved.rules {
        if !uses_old_threshold_form(id, config) {
            continue;
        }

        let Some(spec) = threshold_spec(id) else {
            continue;
        };
        match level_change(spec, config) {
            Ok(change) => plans.push(LevelPlan {
                rule: spec.rule,
                change,
            }),
            Err(reason) => {
                let line = line_of_key(body, resolved.written_key(id));

                return Err(format!("line {line}: {reason}"));
            }
        }
    }

    Ok(plans)
}

fn level_change(spec: &ThresholdSpec, config: &RuleConfig) -> Result<LevelChange, String> {
    let rule = spec.rule;

    if let Some(key) = config
        .option_keys()
        .into_iter()
        .find(|key| !is_migratable_key(spec, key))
    {
        return Err(format!(
            "{rule} has \"{key}\", which the warn/error form does not accept — remove it first"
        ));
    }

    match config.max() {
        Some(max) => max_change(rule, config, max),
        None => severity_only_change(spec, config),
    }
}

fn is_migratable_key(spec: &ThresholdSpec, key: &str) -> bool {
    matches!(key, "include" | "max" | "severity") || spec.extra_keys.contains(&key)
}

fn max_change(rule: &str, config: &RuleConfig, max: &Value) -> Result<LevelChange, String> {
    if !is_positive_integer(max) {
        return Err(format!(
            "max on {rule} is {max}, not a positive integer — fix it by hand first"
        ));
    }

    match config.written_severity() {
        None | Some("error") => Ok(LevelChange::RenameMax { to: Level::Error }),
        Some("warn") => Ok(LevelChange::RenameMax { to: Level::Warn }),
        Some("off") => Ok(LevelChange::TurnOff),
        Some(other) => Err(bad_severity(rule, other)),
    }
}

fn is_positive_integer(value: &Value) -> bool {
    value.as_u64().is_some_and(|number| number >= 1)
}

fn severity_only_change(spec: &ThresholdSpec, config: &RuleConfig) -> Result<LevelChange, String> {
    let limit = spec.default;

    match config.written_severity() {
        None => Ok(LevelChange::InsertLevel { limit }),
        Some("error") => Ok(LevelChange::ReplaceSeverity {
            to: Level::Error,
            limit,
        }),
        Some("warn") => Ok(LevelChange::ReplaceSeverity {
            to: Level::Warn,
            limit,
        }),
        Some("off") => Ok(LevelChange::TurnOff),
        Some(other) => Err(bad_severity(spec.rule, other)),
    }
}

fn bad_severity(rule: &str, severity: &str) -> String {
    format!("severity on {rule} is \"{severity}\", not error, warn or off — fix it by hand first")
}
