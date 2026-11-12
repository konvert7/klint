use crate::config::RuleConfig;
use crate::rules::thresholds::{Limits, ThresholdSpec, parse_threshold_entries};
use crate::rules::{ParsedOptions, RuleOptions, ScopedOptions};
use serde_json::Value;

pub(crate) struct FunctionLengthOptions {
    pub limits: Limits,
    pub skip_blank_lines: bool,
}

const DEFAULT_MAX: u32 = 50;

pub(in crate::rules) const THRESHOLD_SPEC: ThresholdSpec = ThresholdSpec {
    rule: "function-length",
    default: DEFAULT_MAX,
    extra_keys: &["skipBlankLines"],
};

pub(in crate::rules) fn function_length_options(
    _rule: &'static str,
    config: &RuleConfig,
) -> ParsedOptions {
    let (entries, problems) = parse_threshold_entries(&THRESHOLD_SPEC, config, |entry| {
        parse_skip_blank_lines(THRESHOLD_SPEC.rule, entry.skip_blank_lines())
    });
    let scopes = entries
        .into_iter()
        .map(|entry| ScopedOptions {
            include: entry.include,
            options: RuleOptions::FunctionLength(FunctionLengthOptions {
                limits: entry.limits,
                skip_blank_lines: entry.extra,
            }),
        })
        .collect();
    ParsedOptions { scopes, problems }
}

fn parse_skip_blank_lines(rule: &str, value: Option<&Value>) -> (bool, Vec<String>) {
    match value {
        None => (false, Vec::new()),
        Some(Value::Bool(enabled)) => (*enabled, Vec::new()),
        Some(value) => (false, vec![invalid_skip_blank_lines_problem(rule, value)]),
    }
}

fn invalid_skip_blank_lines_problem(rule: &str, value: &Value) -> String {
    format!("Rule \"{rule}\" has skipBlankLines {value}, which is not a boolean, so it uses false.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::thresholds::Level;

    const RULE: &str = "function-length";

    fn options_of(config: RuleConfig) -> (Vec<(Limits, bool)>, Vec<String>) {
        let parsed = function_length_options(RULE, &config);
        let scopes = parsed
            .scopes
            .into_iter()
            .map(|scope| match scope.options {
                RuleOptions::FunctionLength(options) => (options.limits, options.skip_blank_lines),
                _ => panic!("function-length options expected"),
            })
            .collect();
        (scopes, parsed.problems)
    }

    fn parse_options(yaml: &str) -> (Vec<(Limits, bool)>, Vec<String>) {
        options_of(serde_yaml::from_str(yaml).expect("valid rule config"))
    }

    fn parse_v1_options(yaml: &str) -> (Vec<(Limits, bool)>, Vec<String>) {
        let config: RuleConfig = serde_yaml::from_str(yaml).expect("valid rule config");
        options_of(config.into_v1())
    }

    #[test]
    fn bare_severity_uses_fifty_and_no_blank_line_skipping_without_problems() {
        assert_eq!(
            parse_options("error"),
            (vec![(Limits::at(Level::Error, 50), false)], vec![])
        );
    }

    #[test]
    fn valid_max_and_boolean_skip_blank_lines_are_used_without_problems() {
        assert_eq!(
            parse_options("{max: 40, skipBlankLines: true}"),
            (vec![(Limits::at(Level::Error, 40), true)], vec![])
        );
    }

    #[test]
    fn string_skip_blank_lines_is_one_problem_and_uses_false() {
        let (scopes, problems) = parse_v1_options("{skipBlankLines: \"yes\"}");
        assert_eq!(scopes, vec![(Limits::at(Level::Error, 50), false)]);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("\"yes\""));
        assert!(problems[0].contains("function-length"));
    }

    #[test]
    fn number_skip_blank_lines_is_one_problem_and_uses_false() {
        let (scopes, problems) = parse_v1_options("{skipBlankLines: 1}");
        assert_eq!(scopes, vec![(Limits::at(Level::Error, 50), false)]);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains('1'));
    }

    #[test]
    fn invalid_max_and_invalid_skip_blank_lines_are_two_problems() {
        let (scopes, problems) = parse_options("{max: 0, skipBlankLines: 1}");
        assert_eq!(scopes, vec![(Limits::at(Level::Error, 50), false)]);
        assert_eq!(problems.len(), 2);
    }

    #[test]
    fn warn_and_error_with_skip_blank_lines_keep_both_limits() {
        assert_eq!(
            parse_options("{warn: 60, error: 80, skipBlankLines: true}"),
            (vec![(Limits::new(Some(60), Some(80)), true)], vec![])
        );
    }

    #[test]
    fn list_entries_keep_their_include_and_skip_flag() {
        let config: RuleConfig =
            serde_yaml::from_str("[{error: 5, include: [\"src/**\"], skipBlankLines: true}]")
                .expect("valid rule config");
        let parsed = function_length_options(RULE, &config);
        assert!(parsed.problems.is_empty());
        assert_eq!(parsed.scopes.len(), 1);
        assert_eq!(
            parsed.scopes[0].include.as_deref(),
            Some(&["src/**".to_string()][..])
        );
    }
}
