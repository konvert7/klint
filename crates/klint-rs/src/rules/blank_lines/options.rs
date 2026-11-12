use crate::config::RuleConfig;
use crate::rules::{ParsedOptions, RuleOptions};
use crate::syntax::{Keyword, KeywordSet};
use serde_json::Value;

const AFTER_RULE: &str = "blank-line-after-block";

pub(crate) struct BlankLineOptions {
    pub statements: KeywordSet,
    pub allow_setup: bool,
}

pub(in crate::rules) fn blank_line_options(
    rule: &'static str,
    config: &RuleConfig,
) -> ParsedOptions {
    let is_after = rule == AFTER_RULE;
    let default_statements = if is_after {
        KeywordSet::ALL_AFTER
    } else {
        KeywordSet::ALL_BEFORE
    };
    let (statements, problems) = parse_statements(rule, config.statements(), default_statements);
    let allow_setup = is_after
        || config
            .allow_setup()
            .and_then(Value::as_bool)
            .unwrap_or(true);
    ParsedOptions {
        options: RuleOptions::BlankLines(BlankLineOptions {
            statements,
            allow_setup,
        }),
        problems,
    }
}

fn parse_statements(
    rule: &str,
    value: Option<&Value>,
    default: KeywordSet,
) -> (KeywordSet, Vec<String>) {
    let Some(Value::Array(elements)) = value else {
        return (default, Vec::new());
    };
    let mut set = KeywordSet::default();
    let mut problems = Vec::new();

    for element in elements {
        match element.as_str().map(|word| (word, Keyword::parse(word))) {
            Some((_, Some(Keyword::Return))) if rule == AFTER_RULE => {
                problems.push(return_on_after_rule_problem());
            }
            Some((_, Some(keyword))) => set = set.with(keyword),
            Some((word, None)) => problems.push(unknown_statement_problem(rule, word)),
            None => problems.push(unknown_statement_problem(rule, &element.to_string())),
        }
    }

    (set, problems)
}

fn unknown_statement_problem(rule: &str, word: &str) -> String {
    format!("Rule \"{rule}\" lists unknown statement \"{word}\" in statements, so it is ignored.")
}

fn return_on_after_rule_problem() -> String {
    format!(
        "Rule \"{AFTER_RULE}\" lists \"return\" in statements, but only blank-line-before-control-flow supports it, so it is ignored."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RuleConfig;
    use crate::rules::RuleOptions;
    use crate::syntax::{Keyword, KeywordSet};

    const BEFORE: &str = "blank-line-before-control-flow";
    const AFTER: &str = "blank-line-after-block";

    struct Parsed {
        statements: KeywordSet,
        allow_setup: bool,
        problems: Vec<String>,
    }

    fn parse_options(rule: &'static str, yaml: &str) -> Parsed {
        let config: RuleConfig = serde_yaml::from_str(yaml).expect("valid rule config");
        let parsed = blank_line_options(rule, &config);
        let RuleOptions::BlankLines(options) = parsed.options else {
            panic!("blank-line options expected");
        };
        Parsed {
            statements: options.statements,
            allow_setup: options.allow_setup,
            problems: parsed.problems,
        }
    }

    #[test]
    fn bare_severity_gives_every_before_keyword_and_allows_setup() {
        let parsed = parse_options(BEFORE, "error");
        assert_eq!(parsed.statements, KeywordSet::ALL_BEFORE);
        assert!(parsed.allow_setup);
        assert!(parsed.problems.is_empty());
    }

    #[test]
    fn unknown_statement_keyword_is_one_problem_and_the_rest_apply() {
        let parsed = parse_options(BEFORE, "{statements: [if, bogus]}");
        assert_eq!(parsed.statements, KeywordSet::default().with(Keyword::If));
        assert!(parsed.allow_setup);
        assert_eq!(parsed.problems.len(), 1);
        assert!(parsed.problems[0].contains("bogus"));
    }

    #[test]
    fn return_on_the_after_rule_is_one_problem_and_the_rest_apply() {
        let parsed = parse_options(AFTER, "{statements: [return, for]}");
        assert_eq!(parsed.statements, KeywordSet::default().with(Keyword::For));
        assert!(parsed.allow_setup);
        assert_eq!(parsed.problems.len(), 1);
        assert!(parsed.problems[0].contains("return"));
    }

    #[test]
    fn match_and_switch_both_select_switch_and_return_is_kept_on_the_before_rule() {
        let parsed = parse_options(BEFORE, "{statements: [match, switch, return]}");
        assert_eq!(
            parsed.statements,
            KeywordSet::default()
                .with(Keyword::Switch)
                .with(Keyword::Return)
        );
        assert!(parsed.allow_setup);
        assert!(parsed.problems.is_empty());
    }

    #[test]
    fn empty_statements_list_selects_nothing_on_the_after_rule() {
        let parsed = parse_options(AFTER, "{statements: []}");
        assert_eq!(parsed.statements, KeywordSet::default());
        assert!(parsed.allow_setup);
        assert!(parsed.problems.is_empty());
    }

    #[test]
    fn non_array_statements_falls_back_to_every_keyword_silently() {
        let parsed = parse_options(BEFORE, "{statements: \"if\"}");
        assert_eq!(parsed.statements, KeywordSet::ALL_BEFORE);
        assert!(parsed.allow_setup);
        assert!(parsed.problems.is_empty());
    }

    #[test]
    fn non_boolean_allow_setup_falls_back_to_true_silently() {
        let parsed = parse_options(BEFORE, "{allowSetup: \"yes\"}");
        assert_eq!(parsed.statements, KeywordSet::ALL_BEFORE);
        assert!(parsed.allow_setup);
        assert!(parsed.problems.is_empty());
    }

    #[test]
    fn allow_setup_false_is_read_on_the_before_rule() {
        let parsed = parse_options(BEFORE, "{allowSetup: false}");
        assert_eq!(parsed.statements, KeywordSet::ALL_BEFORE);
        assert!(!parsed.allow_setup);
        assert!(parsed.problems.is_empty());
    }

    #[test]
    fn allow_setup_true_on_the_after_rule_keeps_every_block_keyword() {
        let parsed = parse_options(AFTER, "{allowSetup: true}");
        assert_eq!(parsed.statements, KeywordSet::ALL_AFTER);
        assert!(parsed.allow_setup);
        assert!(parsed.problems.is_empty());
    }
}
