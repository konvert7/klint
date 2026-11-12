use crate::config::RuleConfig;
use crate::rules::{ParsedOptions, RuleOptions};
use serde_json::Value;

pub(crate) struct ComplexityOptions {
    pub max: u32,
}

const DEFAULT_MAX: u32 = 15;

pub(in crate::rules) fn cognitive_complexity_options(
    rule: &'static str,
    config: &RuleConfig,
) -> ParsedOptions {
    let (max, problems) = parse_max(rule, config.max(), DEFAULT_MAX);
    ParsedOptions {
        options: RuleOptions::CognitiveComplexity(ComplexityOptions { max }),
        problems,
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
    use crate::config::RuleConfig;
    use crate::rules::RuleOptions;

    const RULE: &str = "cognitive-complexity";

    fn parse_options(yaml: &str) -> (u32, Vec<String>) {
        let config: RuleConfig = serde_yaml::from_str(yaml).expect("valid rule config");
        let parsed = cognitive_complexity_options(RULE, &config);
        let RuleOptions::CognitiveComplexity(options) = parsed.options else {
            panic!("complexity options expected");
        };
        (options.max, parsed.problems)
    }

    #[test]
    fn absent_max_is_fifteen_without_a_problem() {
        assert_eq!(parse_options("error"), (15, Vec::new()));
    }

    #[test]
    fn integer_max_is_used_without_a_problem() {
        assert_eq!(parse_options("{max: 14}"), (14, Vec::new()));
    }

    #[test]
    fn zero_max_is_one_problem_and_uses_fifteen() {
        let (max, problems) = parse_options("{max: 0}");
        assert_eq!(max, 15);
        assert_eq!(
            problems,
            vec![
                "Rule \"cognitive-complexity\" has max 0, which is not an integer of at least 1, so it uses 15."
                    .to_string()
            ]
        );
    }

    #[test]
    fn fractional_max_is_one_problem_and_uses_fifteen() {
        let (max, problems) = parse_options("{max: 2.5}");
        assert_eq!(max, 15);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("max 2.5"));
    }

    #[test]
    fn string_max_is_one_problem_and_uses_fifteen() {
        let (max, problems) = parse_options("{max: \"x\"}");
        assert_eq!(max, 15);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("max \"x\""));
    }

    #[test]
    fn max_above_u32_is_clamped() {
        assert_eq!(parse_options("{max: 4294967296}"), (u32::MAX, Vec::new()));
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
