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
    let (max, problems) = parse_max(rule, config.max());
    ParsedOptions {
        options: RuleOptions::CognitiveComplexity(ComplexityOptions { max }),
        problems,
    }
}

fn parse_max(rule: &str, value: Option<&Value>) -> (u32, Vec<String>) {
    let Some(value) = value else {
        return (DEFAULT_MAX, Vec::new());
    };
    match value.as_u64().filter(|max| *max >= 1) {
        Some(max) => (u32::try_from(max).unwrap_or(u32::MAX), Vec::new()),
        None => (DEFAULT_MAX, vec![invalid_max_problem(rule, value)]),
    }
}

fn invalid_max_problem(rule: &str, value: &Value) -> String {
    format!(
        "Rule \"{rule}\" has max {value}, which is not an integer of at least 1, so it uses {DEFAULT_MAX}."
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
}
