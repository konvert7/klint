use super::{Report, RuleRun, line_fix};
use crate::config::RuleConfig;
use crate::output::Violation;
use crate::syntax::{
    scan_consecutive_array_push_from_tree, scan_nested_template_literals_from_tree,
    scan_string_match_from_tree, scan_sync_in_async_from_tree, scan_unguarded_json_parse_from_tree,
};

pub(super) fn run_no_string_match(config: &RuleConfig, run: RuleRun<'_>) -> Vec<Violation> {
    run.check(
        "no-string-match",
        config,
        scan_string_match_from_tree,
        |record, content| {
            let replacement = format!("new RegExp({}).exec({})", record.regex, record.receiver);
            Report {
                line: record.line,
                message: format!(
                    "Use RegExp.exec() instead of String.match() for non-global regexes — use {replacement} instead."
                ),
                fix: line_fix(
                    content,
                    record.start_row,
                    record.end_row,
                    record.start_byte,
                    record.end_byte,
                    &replacement,
                ),
            }
        },
    )
}

pub(super) fn run_no_nested_template_literals(
    config: &RuleConfig,
    run: RuleRun<'_>,
) -> Vec<Violation> {
    run.check(
        "no-nested-template-literals",
        config,
        |root, _source| scan_nested_template_literals_from_tree(root),
        |record, _content| Report {
            line: record.line,
            message:
                "Nested template literal — extract the inner template to a variable to improve readability."
                    .to_string(),
            fix: None,
        },
    )
}

pub(super) fn run_no_consecutive_array_push(
    config: &RuleConfig,
    run: RuleRun<'_>,
) -> Vec<Violation> {
    run.check(
        "no-consecutive-array-push",
        config,
        scan_consecutive_array_push_from_tree,
        |record, _content| Report {
            line: record.line,
            message: format!(
                "{} consecutive .push() calls on `{}` — combine into a single .push(a, b, …) call.",
                record.count, record.receiver
            ),
            fix: None,
        },
    )
}

pub(super) fn run_no_unguarded_json_parse(config: &RuleConfig, run: RuleRun<'_>) -> Vec<Violation> {
    run.check(
        "no-unguarded-json-parse",
        config,
        scan_unguarded_json_parse_from_tree,
        |record, _content| Report {
            line: record.line,
            message:
                "JSON.parse() called without a surrounding try/catch — a malformed payload will throw an unhandled exception."
                    .to_string(),
            fix: None,
        },
    )
}

pub(super) fn run_no_sync_in_async(config: &RuleConfig, run: RuleRun<'_>) -> Vec<Violation> {
    run.check(
        "no-sync-in-async",
        config,
        scan_sync_in_async_from_tree,
        |record, _content| Report {
            line: record.line,
            message: format!(
                "{}() blocks the event loop inside an async function — use the async equivalent from node:fs/promises.",
                record.name
            ),
            fix: None,
        },
    )
}
