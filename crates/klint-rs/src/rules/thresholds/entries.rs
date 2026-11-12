use super::{Level, Limits, ThresholdForm, ThresholdSpec, threshold_form};
use crate::config::RuleConfig;
use serde_json::Value;

pub(in crate::rules) struct ThresholdEntry<T> {
    pub(in crate::rules) include: Option<Vec<String>>,
    pub(in crate::rules) limits: Limits,
    pub(in crate::rules) extra: T,
}

type Entries<T> = (Vec<ThresholdEntry<T>>, Vec<String>);

pub(in crate::rules) fn parse_threshold_entries<T>(
    spec: &ThresholdSpec,
    config: &RuleConfig,
    extra: impl Fn(&RuleConfig) -> (T, Vec<String>),
) -> Entries<T> {
    match threshold_form(config) {
        ThresholdForm::Bare => parse_bare_entry(spec, config, extra),
        ThresholdForm::Old => parse_old_entry(spec, config, extra),
        ThresholdForm::Levels => parse_levels_form(spec, config, &[], extra),
        ThresholdForm::Mixed => parse_mixed(spec, config, extra),
        ThresholdForm::List => parse_list_entries(spec, config.list_items().unwrap_or(&[]), extra),
    }
}

fn parse_bare_entry<T>(
    spec: &ThresholdSpec,
    config: &RuleConfig,
    extra: impl Fn(&RuleConfig) -> (T, Vec<String>),
) -> Entries<T> {
    let level = if config.severity() == "warn" {
        Level::Warn
    } else {
        Level::Error
    };
    let (extra_value, problems) = extra(config);
    let entry = ThresholdEntry {
        include: None,
        limits: Limits::at(level, spec.default),
        extra: extra_value,
    };
    (vec![entry], problems)
}

fn parse_old_entry<T>(
    spec: &ThresholdSpec,
    config: &RuleConfig,
    extra: impl Fn(&RuleConfig) -> (T, Vec<String>),
) -> Entries<T> {
    let (limit, mut problems) = parse_max(spec.rule, config.max(), spec.default);
    let level = if config.written_severity() == Some("warn") {
        Level::Warn
    } else {
        Level::Error
    };
    let (extra_value, extra_problems) = extra(config);
    problems.extend(extra_problems);
    let entry = ThresholdEntry {
        include: config.include().map(<[String]>::to_vec),
        limits: Limits::at(level, limit),
        extra: extra_value,
    };
    (vec![entry], problems)
}

fn parse_mixed<T>(
    spec: &ThresholdSpec,
    config: &RuleConfig,
    extra: impl Fn(&RuleConfig) -> (T, Vec<String>),
) -> Entries<T> {
    let (entries, mut problems) = parse_levels_form(spec, config, &["max", "severity"], extra);
    let mixed = format!(
        "{} mixes max/severity with warn/error — max and severity are ignored.",
        label(spec.rule, None)
    );
    problems.insert(0, mixed);
    (entries, problems)
}

fn parse_levels_form<T>(
    spec: &ThresholdSpec,
    config: &RuleConfig,
    ignored: &[&str],
    extra: impl Fn(&RuleConfig) -> (T, Vec<String>),
) -> Entries<T> {
    let top_label = label(spec.rule, None);
    let (entry, problems) = parse_levels_entry(spec, config, &top_label, ignored, extra);
    (entry.into_iter().collect(), problems)
}

fn parse_levels_entry<T>(
    spec: &ThresholdSpec,
    config: &RuleConfig,
    label: &str,
    ignored: &[&str],
    extra: impl Fn(&RuleConfig) -> (T, Vec<String>),
) -> (Option<ThresholdEntry<T>>, Vec<String>) {
    if let Some(key) = unknown_key(spec, config, ignored) {
        return (
            None,
            vec![format!(
                "{label} has unknown key \"{key}\", so it is ignored."
            )],
        );
    }

    if config.warn().is_none() && config.error().is_none() {
        return (
            None,
            vec![format!(
                "{label} has neither warn nor error, so it is ignored."
            )],
        );
    }

    let (warn, error) = match read_limits(label, config) {
        Ok(limits) => limits,
        Err(problem) => return (None, vec![problem]),
    };
    let mut problems = error_below_warn_problems(label, warn, error);
    let (extra_value, extra_problems) = extra(config);
    problems.extend(extra_problems);
    let entry = ThresholdEntry {
        include: config.include().map(<[String]>::to_vec),
        limits: Limits::new(warn, error),
        extra: extra_value,
    };
    (Some(entry), problems)
}

fn unknown_key<'a>(
    spec: &ThresholdSpec,
    config: &'a RuleConfig,
    ignored: &[&str],
) -> Option<&'a str> {
    config
        .option_keys()
        .into_iter()
        .find(|key| !is_level_key(spec, key, ignored))
}

fn is_level_key(spec: &ThresholdSpec, key: &str, ignored: &[&str]) -> bool {
    matches!(key, "warn" | "error" | "include")
        || spec.extra_keys.contains(&key)
        || ignored.contains(&key)
}

fn read_limits(label: &str, config: &RuleConfig) -> Result<(Option<u32>, Option<u32>), String> {
    let warn = limit_value(label, "warn", config.warn())?;
    let error = limit_value(label, "error", config.error())?;
    Ok((warn, error))
}

fn limit_value(label: &str, key: &str, value: Option<&Value>) -> Result<Option<u32>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    match value.as_u64().filter(|limit| *limit >= 1) {
        Some(limit) => Ok(Some(u32::try_from(limit).unwrap_or(u32::MAX))),
        None => Err(format!(
            "{label} has {key} {value}, which is not an integer of at least 1, so it is ignored."
        )),
    }
}

fn error_below_warn_problems(label: &str, warn: Option<u32>, error: Option<u32>) -> Vec<String> {
    match (warn, error) {
        (Some(warn), Some(error)) if error < warn => vec![format!(
            "{label} has error {error} below warn {warn}, so it runs with error {error} only."
        )],
        _ => Vec::new(),
    }
}

fn parse_list_entries<T>(
    spec: &ThresholdSpec,
    items: &[Value],
    extra: impl Fn(&RuleConfig) -> (T, Vec<String>),
) -> Entries<T> {
    let mut entries = Vec::new();
    let mut problems = Vec::new();

    for (index, item) in items.iter().enumerate() {
        let (entry, item_problems) = parse_list_item(spec, item, index + 1, &extra);
        entries.extend(entry);
        problems.extend(item_problems);
    }

    (entries, problems)
}

fn parse_list_item<T>(
    spec: &ThresholdSpec,
    item: &Value,
    number: usize,
    extra: impl Fn(&RuleConfig) -> (T, Vec<String>),
) -> (Option<ThresholdEntry<T>>, Vec<String>) {
    let entry_label = label(spec.rule, Some(number));

    if !item.is_object() {
        return (
            None,
            vec![format!("{entry_label} is not an object, so it is ignored.")],
        );
    }

    match serde_json::from_value::<RuleConfig>(item.clone()) {
        Ok(entry_config) => parse_levels_entry(spec, &entry_config, &entry_label, &[], extra),
        Err(_) => (
            None,
            vec![format!(
                "{entry_label} is not a valid entry, so it is ignored."
            )],
        ),
    }
}

fn label(rule: &str, entry: Option<usize>) -> String {
    match entry {
        Some(number) => format!("Rule \"{rule}\" entry {number}"),
        None => format!("Rule \"{rule}\""),
    }
}

pub(in crate::rules) fn parse_max(
    rule: &str,
    value: Option<&Value>,
    default: u32,
) -> (u32, Vec<String>) {
    let Some(value) = value else {
        return (default, Vec::new());
    };
    match value.as_u64().filter(|max| *max >= 1) {
        Some(max) => (u32::try_from(max).unwrap_or(u32::MAX), Vec::new()),
        None => (default, vec![invalid_max_problem(rule, value, default)]),
    }
}

fn invalid_max_problem(rule: &str, value: &Value, default: u32) -> String {
    format!(
        "Rule \"{rule}\" has max {value}, which is not an integer of at least 1, so it uses {default}."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: ThresholdSpec = ThresholdSpec {
        rule: "cognitive-complexity",
        default: 15,
        extra_keys: &[],
    };

    type Shown = (Vec<(Option<Vec<String>>, Limits)>, Vec<String>);

    fn entries_of(yaml: &str) -> Shown {
        let config: RuleConfig = serde_yaml::from_str(yaml).expect("valid rule config");
        let (entries, problems) = parse_threshold_entries(&SPEC, &config, |_| ((), Vec::new()));
        let shown = entries
            .into_iter()
            .map(|entry| (entry.include, entry.limits))
            .collect();
        (shown, problems)
    }

    fn error_only(limit: u32) -> Limits {
        Limits::at(Level::Error, limit)
    }

    fn problem(text: &str) -> Vec<String> {
        vec![text.to_string()]
    }

    #[test]
    fn bare_error_and_bare_warn_use_the_default_at_their_level() {
        assert_eq!(entries_of("error"), (vec![(None, error_only(15))], vec![]));
        assert_eq!(
            entries_of("warn"),
            (vec![(None, Limits::at(Level::Warn, 15))], vec![])
        );
    }

    #[test]
    fn zero_max_in_the_old_form_is_one_problem_and_uses_the_default() {
        assert_eq!(
            entries_of("{max: 0, severity: warn}"),
            (
                vec![(None, Limits::at(Level::Warn, 15))],
                problem(
                    "Rule \"cognitive-complexity\" has max 0, which is not an integer of at least 1, so it uses 15."
                )
            )
        );
    }

    #[test]
    fn error_below_warn_warns_and_keeps_error_only() {
        assert_eq!(
            entries_of("{warn: 20, error: 10}"),
            (
                vec![(None, error_only(10))],
                problem(
                    "Rule \"cognitive-complexity\" has error 10 below warn 20, so it runs with error 10 only."
                )
            )
        );
    }

    #[test]
    fn list_entry_that_is_not_an_object_is_skipped_with_its_number() {
        assert_eq!(
            entries_of("[7, {error: 15}]"),
            (
                vec![(None, error_only(15))],
                problem(
                    "Rule \"cognitive-complexity\" entry 1 is not an object, so it is ignored."
                )
            )
        );
    }

    #[test]
    fn list_entry_with_an_unknown_key_is_skipped() {
        assert_eq!(
            entries_of("[{error: 5, bogus: 1}]"),
            (
                vec![],
                problem(
                    "Rule \"cognitive-complexity\" entry 1 has unknown key \"bogus\", so it is ignored."
                )
            )
        );
    }

    #[test]
    fn include_without_a_level_is_neither_warn_nor_error() {
        assert_eq!(
            entries_of("{include: [\"src/**\"]}"),
            (
                vec![],
                problem(
                    "Rule \"cognitive-complexity\" has neither warn nor error, so it is ignored."
                )
            )
        );
    }

    #[test]
    fn mixing_old_and_new_keys_warns_once_and_keeps_the_levels() {
        assert_eq!(
            entries_of("{max: 3, severity: warn, error: 10}"),
            (
                vec![(None, error_only(10))],
                problem(
                    "Rule \"cognitive-complexity\" mixes max/severity with warn/error — max and severity are ignored."
                )
            )
        );
    }

    #[test]
    fn list_entry_with_max_reports_max_as_an_unknown_key() {
        assert_eq!(
            entries_of("[{max: 10}]"),
            (
                vec![],
                problem(
                    "Rule \"cognitive-complexity\" entry 1 has unknown key \"max\", so it is ignored."
                )
            )
        );
    }

    #[test]
    fn empty_list_has_no_entries_and_no_problems() {
        assert_eq!(entries_of("[]"), (vec![], vec![]));
    }

    #[test]
    fn shared_parse_max_absent_value_uses_the_given_default() {
        assert_eq!(
            parse_max("function-length", None, 50),
            (50, Vec::<String>::new())
        );
    }

    #[test]
    fn shared_parse_max_integer_value_is_used_with_any_default() {
        assert_eq!(
            parse_max("function-length", Some(&serde_json::json!(40)), 50),
            (40, Vec::<String>::new())
        );
    }

    #[test]
    fn shared_parse_max_zero_is_one_problem_naming_the_given_default() {
        assert_eq!(
            parse_max("function-length", Some(&serde_json::json!(0)), 50),
            (
                50,
                vec![
                    "Rule \"function-length\" has max 0, which is not an integer of at least 1, so it uses 50."
                        .to_string()
                ]
            )
        );
    }

    #[test]
    fn shared_parse_max_negative_fractional_and_string_values_are_one_problem_each() {
        let values = [
            (serde_json::json!(-1), "-1"),
            (serde_json::json!(2.5), "2.5"),
            (serde_json::json!("x"), "\"x\""),
        ];

        for (value, text) in values {
            let (max, problems) = parse_max("function-length", Some(&value), 50);
            assert_eq!(max, 50);
            assert_eq!(problems.len(), 1);
            assert!(problems[0].contains(text));
        }
    }

    #[test]
    fn shared_parse_max_clamps_values_above_u32_for_any_rule() {
        assert_eq!(
            parse_max(
                "cognitive-complexity",
                Some(&serde_json::json!(4294967296_u64)),
                15
            ),
            (u32::MAX, Vec::<String>::new())
        );
    }
}
