use crate::config::RuleConfig;
use crate::rules::complexity::parse_max;
use crate::rules::{ParsedOptions, RuleOptions};
use serde_json::Value;

pub(crate) struct FunctionLengthOptions {
    pub max: u32,
    pub skip_blank_lines: bool,
}

const DEFAULT_MAX: u32 = 50;

pub(in crate::rules) fn function_length_options(
    rule: &'static str,
    config: &RuleConfig,
) -> ParsedOptions {
    let (max, mut problems) = parse_max(rule, config.max(), DEFAULT_MAX);
    let (skip_blank_lines, skip_problems) = parse_skip_blank_lines(rule, config.skip_blank_lines());
    problems.extend(skip_problems);
    ParsedOptions {
        options: RuleOptions::FunctionLength(FunctionLengthOptions {
            max,
            skip_blank_lines,
        }),
        problems,
    }
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
    use crate::config::RuleConfig;
    use crate::rules::RuleOptions;

    const RULE: &str = "function-length";

    fn parse_options(yaml: &str) -> (u32, bool, Vec<String>) {
        let config: RuleConfig = serde_yaml::from_str(yaml).expect("valid rule config");
        let parsed = function_length_options(RULE, &config);
        let RuleOptions::FunctionLength(options) = parsed.options else {
            panic!("function-length options expected");
        };
        (options.max, options.skip_blank_lines, parsed.problems)
    }

    #[test]
    fn bare_severity_uses_fifty_and_no_blank_line_skipping_without_problems() {
        let (max, skip_blank_lines, problems) = parse_options("error");
        assert_eq!((max, skip_blank_lines), (50, false));
        assert!(problems.is_empty());
    }

    #[test]
    fn valid_max_and_boolean_skip_blank_lines_are_used_without_problems() {
        let (max, skip_blank_lines, problems) = parse_options("{max: 40, skipBlankLines: true}");
        assert_eq!((max, skip_blank_lines), (40, true));
        assert!(problems.is_empty());
    }

    #[test]
    fn string_skip_blank_lines_is_one_problem_and_uses_false() {
        let (max, skip_blank_lines, problems) = parse_options("{skipBlankLines: \"yes\"}");
        assert_eq!((max, skip_blank_lines), (50, false));
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("\"yes\""));
        assert!(problems[0].contains("function-length"));
    }

    #[test]
    fn number_skip_blank_lines_is_one_problem_and_uses_false() {
        let (max, skip_blank_lines, problems) = parse_options("{skipBlankLines: 1}");
        assert_eq!((max, skip_blank_lines), (50, false));
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains('1'));
    }

    #[test]
    fn invalid_max_and_invalid_skip_blank_lines_are_two_problems() {
        let (max, skip_blank_lines, problems) = parse_options("{max: 0, skipBlankLines: 1}");
        assert_eq!((max, skip_blank_lines), (50, false));
        assert_eq!(problems.len(), 2);
    }
}
