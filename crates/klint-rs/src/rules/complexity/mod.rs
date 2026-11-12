mod options;
mod schema_shapes;

pub(crate) use options::ComplexityOptions;
pub(super) use options::{THRESHOLD_SPEC, cognitive_complexity_options};
pub(super) use schema_shapes::{cognitive_complexity_schema, cognitive_complexity_v1_schema};

use super::{Report, RuleOptions, RuleScan};
use crate::syntax::{function_name, sort_by_position};

pub(super) fn run_cognitive_complexity(scan: RuleScan<'_>) -> Vec<Report> {
    let RuleOptions::CognitiveComplexity(options) = scan.options else {
        return Vec::new();
    };

    if scan.index.has_syntax_error {
        return Vec::new();
    }

    let mut scores = scan.index.scores.clone();
    sort_by_position(&mut scores);
    scores
        .into_iter()
        .filter_map(|function| {
            let (level, limit) = options.limits.exceeded_by(function.score)?;
            Some(Report {
                line: function.node.start_position().row + 1,
                message: complexity_message(
                    &function_name(function.node, scan.source, scan.language),
                    function.score,
                    limit,
                ),
                fix: None,
                level: Some(level),
            })
        })
        .collect()
}

fn complexity_message(name: &str, score: u32, max: u32) -> String {
    format!(
        "Function `{name}` has cognitive complexity {score} (max {max}) — split it into smaller functions."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_names_the_function_its_score_and_the_limit() {
        assert_eq!(
            complexity_message("big", 16, 15),
            "Function `big` has cognitive complexity 16 (max 15) — split it into smaller functions."
        );
    }
}
