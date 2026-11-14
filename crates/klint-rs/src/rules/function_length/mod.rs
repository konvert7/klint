mod options;
mod schema_shapes;

pub(crate) use options::FunctionLengthOptions;
pub(super) use options::{THRESHOLD_SPEC, function_length_options};
pub(super) use schema_shapes::{function_length_schema, function_length_v1_schema};

use super::{Report, RuleOptions, RuleScan};
use crate::syntax::{FunctionLength, function_name};

pub(super) fn run_function_length(scan: RuleScan<'_>) -> Vec<Report> {
    let RuleOptions::FunctionLength(options) = scan.options else {
        return Vec::new();
    };

    if scan.index.has_syntax_error {
        return Vec::new();
    }

    let mut lengths = scan.index.function_lengths.clone();
    lengths.sort_by_key(|length| {
        let start = length.node.start_position();
        (start.row, start.column)
    });
    lengths
        .into_iter()
        .filter_map(|length| {
            let count = counted_lines(&length, options.skip_blank_lines);
            let (level, limit) = options.limits.exceeded_by(count)?;
            Some(Report {
                line: length.node.start_position().row + 1,
                message: length_message(
                    &function_name(length.node, scan.source, scan.language),
                    count,
                    limit,
                ),
                fix: None,
                level: Some(level),
                bytes: None,
            })
        })
        .collect()
}

fn counted_lines(length: &FunctionLength<'_>, skip_blank_lines: bool) -> u32 {
    if skip_blank_lines {
        length.non_blank_lines
    } else {
        length.lines
    }
}

fn length_message(name: &str, count: u32, max: u32) -> String {
    format!("Function `{name}` has {count} lines (max {max}) — split it into smaller functions.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::{FunctionLength, parse_source};
    use std::path::Path;

    fn counted(lines: u32, non_blank_lines: u32, skip_blank_lines: bool) -> u32 {
        let tree = parse_source(Path::new("a.ts"), "const a = 1;").expect("snippet parses");
        let length = FunctionLength {
            node: tree.root_node(),
            lines,
            non_blank_lines,
        };
        counted_lines(&length, skip_blank_lines)
    }

    #[test]
    fn all_lines_are_counted_without_skip_blank_lines() {
        assert_eq!(counted(3, 2, false), 3);
    }

    #[test]
    fn only_non_blank_lines_are_counted_with_skip_blank_lines() {
        assert_eq!(counted(3, 2, true), 2);
    }

    #[test]
    fn an_empty_body_counts_zero_with_skip_blank_lines() {
        assert_eq!(counted(0, 0, true), 0);
    }

    #[test]
    fn message_names_a_function_fifty_one_lines_against_fifty() {
        assert_eq!(
            length_message("big", 51, 50),
            "Function `big` has 51 lines (max 50) — split it into smaller functions."
        );
    }

    #[test]
    fn message_names_an_anonymous_function_two_lines_against_one() {
        assert_eq!(
            length_message("<anonymous>", 2, 1),
            "Function `<anonymous>` has 2 lines (max 1) — split it into smaller functions."
        );
    }

    #[test]
    fn message_names_an_operator_four_lines_against_three() {
        assert_eq!(
            length_message("operator +", 4, 3),
            "Function `operator +` has 4 lines (max 3) — split it into smaller functions."
        );
    }
}
