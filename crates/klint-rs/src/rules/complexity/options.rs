use crate::config::RuleConfig;
use crate::rules::thresholds::{Limits, ThresholdSpec, parse_threshold_entries};
use crate::rules::{ParsedOptions, RuleOptions, ScopedOptions};

pub(crate) struct ComplexityOptions {
    pub limits: Limits,
}

const DEFAULT_MAX: u32 = 15;

pub(in crate::rules) const THRESHOLD_SPEC: ThresholdSpec = ThresholdSpec {
    rule: "cognitive-complexity",
    default: DEFAULT_MAX,
    extra_keys: &[],
};

pub(in crate::rules) fn cognitive_complexity_options(
    _rule: &'static str,
    config: &RuleConfig,
) -> ParsedOptions {
    let (entries, problems) =
        parse_threshold_entries(&THRESHOLD_SPEC, config, |_| ((), Vec::new()));
    let scopes = entries
        .into_iter()
        .map(|entry| ScopedOptions {
            include: entry.include,
            options: RuleOptions::CognitiveComplexity(ComplexityOptions {
                limits: entry.limits,
            }),
        })
        .collect();
    ParsedOptions { scopes, problems }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::thresholds::Level;

    const RULE: &str = "cognitive-complexity";

    fn parse_options(yaml: &str) -> (Vec<Limits>, Vec<String>) {
        let config: RuleConfig = serde_yaml::from_str(yaml).expect("valid rule config");
        let parsed = cognitive_complexity_options(RULE, &config);
        let limits = parsed
            .scopes
            .into_iter()
            .map(|scope| match scope.options {
                RuleOptions::CognitiveComplexity(options) => options.limits,
                _ => panic!("complexity options expected"),
            })
            .collect();
        (limits, parsed.problems)
    }

    #[test]
    fn absent_max_is_fifteen_without_a_problem() {
        assert_eq!(
            parse_options("error"),
            (vec![Limits::at(Level::Error, 15)], vec![])
        );
    }

    #[test]
    fn integer_max_is_used_without_a_problem() {
        assert_eq!(
            parse_options("{max: 14}"),
            (vec![Limits::at(Level::Error, 14)], vec![])
        );
    }

    #[test]
    fn zero_max_is_one_problem_and_uses_fifteen() {
        let (limits, problems) = parse_options("{max: 0}");
        assert_eq!(limits, vec![Limits::at(Level::Error, 15)]);
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
        let (limits, problems) = parse_options("{max: 2.5}");
        assert_eq!(limits, vec![Limits::at(Level::Error, 15)]);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("max 2.5"));
    }

    #[test]
    fn string_max_is_one_problem_and_uses_fifteen() {
        let (limits, problems) = parse_options("{max: \"x\"}");
        assert_eq!(limits, vec![Limits::at(Level::Error, 15)]);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("max \"x\""));
    }

    #[test]
    fn max_above_u32_is_clamped() {
        assert_eq!(
            parse_options("{max: 4294967296}"),
            (vec![Limits::at(Level::Error, u32::MAX)], vec![])
        );
    }
}
