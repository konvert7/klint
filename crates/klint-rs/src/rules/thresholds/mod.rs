mod entries;

use super::{complexity, function_length};
use crate::config::RuleConfig;

pub(in crate::rules) use entries::parse_threshold_entries;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Level {
    Warn,
    Error,
}

impl Level {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Level::Warn => "warn",
            Level::Error => "error",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Limits {
    warn: Option<u32>,
    error: Option<u32>,
}

impl Limits {
    pub(crate) fn new(warn: Option<u32>, error: Option<u32>) -> Limits {
        let warn = match (warn, error) {
            (Some(warn), Some(error)) if error <= warn => None,
            _ => warn,
        };
        Limits { warn, error }
    }

    pub(crate) fn at(level: Level, limit: u32) -> Limits {
        match level {
            Level::Warn => Limits::new(Some(limit), None),
            Level::Error => Limits::new(None, Some(limit)),
        }
    }

    pub(crate) fn exceeded_by(&self, value: u32) -> Option<(Level, u32)> {
        if let Some(error) = self.error.filter(|error| value > *error) {
            return Some((Level::Error, error));
        }

        self.warn
            .filter(|warn| value > *warn)
            .map(|warn| (Level::Warn, warn))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ThresholdForm {
    Bare,
    Old,
    Levels,
    Mixed,
    List,
}

pub(crate) struct ThresholdSpec {
    pub(crate) rule: &'static str,
    pub(crate) default: u32,
    pub(crate) extra_keys: &'static [&'static str],
}

pub(crate) fn threshold_form(config: &RuleConfig) -> ThresholdForm {
    if matches!(config, RuleConfig::Severity(_)) {
        return ThresholdForm::Bare;
    }

    if config.list_items().is_some() {
        return ThresholdForm::List;
    }

    if config.read_as_v1() {
        return ThresholdForm::Old;
    }

    match (has_level_key(config), has_old_key(config)) {
        (true, true) => ThresholdForm::Mixed,
        (false, true) => ThresholdForm::Old,
        _ => ThresholdForm::Levels,
    }
}

fn has_level_key(config: &RuleConfig) -> bool {
    config.warn().is_some() || config.error().is_some()
}

fn has_old_key(config: &RuleConfig) -> bool {
    config.max().is_some() || config.written_severity().is_some()
}

pub(crate) fn threshold_severity(config: &RuleConfig) -> &str {
    match threshold_form(config) {
        ThresholdForm::Levels | ThresholdForm::Mixed => "error",
        _ => config.severity(),
    }
}

pub(crate) fn old_form_message(rule: &str) -> String {
    format!(
        "`max` and `severity` on {rule} are the old form — run `klint migrate` to upgrade it in place."
    )
}

const THRESHOLD_SPECS: [&ThresholdSpec; 2] = [
    &complexity::THRESHOLD_SPEC,
    &function_length::THRESHOLD_SPEC,
];

pub(crate) fn threshold_spec(name: &str) -> Option<&'static ThresholdSpec> {
    THRESHOLD_SPECS.into_iter().find(|spec| spec.rule == name)
}

pub(crate) fn accepts_list_form(rule_id: &str) -> bool {
    threshold_spec(rule_id).is_some()
}

pub(crate) fn effective_severity<'a>(name: &str, config: &'a RuleConfig) -> &'a str {
    if threshold_spec(name).is_some() {
        threshold_severity(config)
    } else {
        config.severity()
    }
}

pub(crate) fn uses_old_threshold_form(name: &str, config: &RuleConfig) -> bool {
    threshold_spec(name).is_some() && threshold_form(config) == ThresholdForm::Old
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule_config(yaml: &str) -> RuleConfig {
        serde_yaml::from_str(yaml).expect("valid rule config")
    }

    #[test]
    fn limits_new_drops_warn_when_error_is_not_above_it() {
        let cases = [
            ((Some(60), Some(80)), (Some(60), Some(80))),
            ((Some(20), Some(10)), (None, Some(10))),
            ((Some(5), Some(5)), (None, Some(5))),
            ((Some(10), None), (Some(10), None)),
        ];

        for ((warn, error), (expect_warn, expect_error)) in cases {
            assert_eq!(
                Limits::new(warn, error),
                Limits {
                    warn: expect_warn,
                    error: expect_error
                }
            );
        }
    }

    #[test]
    fn exceeded_by_is_strict_and_checks_error_first() {
        let both = Limits::new(Some(60), Some(80));
        assert_eq!(both.exceeded_by(60), None);
        assert_eq!(both.exceeded_by(61), Some((Level::Warn, 60)));
        assert_eq!(both.exceeded_by(80), Some((Level::Warn, 60)));
        assert_eq!(both.exceeded_by(81), Some((Level::Error, 80)));
        assert_eq!(
            Limits::new(None, Some(5)).exceeded_by(6),
            Some((Level::Error, 5))
        );
        assert_eq!(Limits::new(Some(5), None).exceeded_by(5), None);
    }

    #[test]
    fn threshold_form_follows_the_written_keys() {
        assert_eq!(threshold_form(&rule_config("warn")), ThresholdForm::Bare);
        assert_eq!(
            threshold_form(&rule_config("{max: 10, severity: warn}")),
            ThresholdForm::Old
        );
        assert_eq!(
            threshold_form(&rule_config("{include: [\"src/**\"]}")),
            ThresholdForm::Levels
        );
        assert_eq!(
            threshold_form(&rule_config("{warn: 10, error: 20}")),
            ThresholdForm::Levels
        );
        assert_eq!(
            threshold_form(&rule_config("{max: 3, error: 10}")),
            ThresholdForm::Mixed
        );
        assert_eq!(threshold_form(&rule_config("[]")), ThresholdForm::List);
    }

    #[test]
    fn v1_objects_are_always_the_old_form() {
        let include = rule_config("{include: [\"src/**\"]}").into_v1();
        assert_eq!(threshold_form(&include), ThresholdForm::Old);
        let warn = rule_config("{warn: 10}").into_v1();
        assert_eq!(threshold_form(&warn), ThresholdForm::Old);
    }

    #[test]
    fn threshold_severity_is_error_for_level_forms() {
        assert_eq!(threshold_severity(&rule_config("{warn: 10}")), "error");
        assert_eq!(
            threshold_severity(&rule_config("{max: 3, severity: warn, error: 10}")),
            "error"
        );
        assert_eq!(
            threshold_severity(&rule_config("{max: 3, severity: warn}")),
            "warn"
        );
        assert_eq!(threshold_severity(&rule_config("[]")), "off");
    }
}
