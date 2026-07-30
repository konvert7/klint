use super::{Report, RuleRun, line_fix};
use crate::config::RuleConfig;
use crate::output::Violation;
use crate::syntax::{
    scan_prefer_at_from_tree, scan_prefer_nullish_coalescing_assign_from_tree,
    scan_prefer_string_raw_from_tree, scan_prefer_string_raw_regexp_from_tree,
    scan_prefer_string_replaceall_from_tree, scan_single_char_classes_from_tree,
};

pub(super) fn run_sonar_no_single_char_class(
    config: &RuleConfig,
    run: RuleRun<'_>,
) -> Vec<Violation> {
    run.check(
        "sonar/no-single-char-class",
        config,
        scan_single_char_classes_from_tree,
        |record, content| Report {
            line: record.line,
            message: format!(
                "Character class [{}] contains a single element — remove the brackets.",
                record.class
            ),
            fix: line_fix(
                content,
                record.start_row,
                record.end_row,
                record.start_byte,
                record.end_byte,
                &record.fixed_regex,
            ),
        },
    )
}

pub(super) fn run_sonar_prefer_at(config: &RuleConfig, run: RuleRun<'_>) -> Vec<Violation> {
    run.check(
        "sonar/prefer-at",
        config,
        scan_prefer_at_from_tree,
        |record, content| {
            let replacement = format!("{}.at(-{})", record.base, record.offset);
            Report {
                line: record.line,
                message: format!(
                    "Prefer {} over {}[{}.length - {}] for cleaner negative indexing.",
                    replacement, record.base, record.base, record.offset
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

pub(super) fn run_sonar_prefer_string_replaceall(
    config: &RuleConfig,
    run: RuleRun<'_>,
) -> Vec<Violation> {
    run.check(
        "sonar/prefer-string-replaceall",
        config,
        scan_prefer_string_replaceall_from_tree,
        |record, content| {
            let replacement = format!(
                "{}.replaceAll({}, {})",
                record.receiver, record.pattern_lit, record.replacement
            );
            Report {
                line: record.line,
                message: format!(
                    "Prefer `{}.replaceAll({}, ...)` over `.replace(/{}/g, ...)` — replaceAll() with a string is clearer and avoids regex escaping pitfalls.",
                    record.receiver, record.pattern_lit, record.pattern
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

pub(super) fn run_sonar_prefer_string_raw_regexp(
    config: &RuleConfig,
    run: RuleRun<'_>,
) -> Vec<Violation> {
    run.check(
        "sonar/prefer-string-raw-regexp",
        config,
        scan_prefer_string_raw_regexp_from_tree,
        |record, content| Report {
            line: record.line,
            message:
                "Use String.raw`...` for RegExp template argument to avoid double backslashes (Sonar S7780)."
                    .to_string(),
            fix: line_fix(
                content,
                record.start_row,
                record.end_row,
                record.start_byte,
                record.end_byte,
                &record.fixed_arg,
            ),
        },
    )
}

pub(super) fn run_sonar_prefer_string_raw(config: &RuleConfig, run: RuleRun<'_>) -> Vec<Violation> {
    run.check(
        "sonar/prefer-string-raw",
        config,
        scan_prefer_string_raw_from_tree,
        |record, content| Report {
            line: record.line,
            message:
                "String literal with escaped backslashes — use String.raw`...` for clarity (Sonar S6535)."
                    .to_string(),
            fix: line_fix(
                content,
                record.start_row,
                record.end_row,
                record.start_byte,
                record.end_byte,
                &record.fixed,
            ),
        },
    )
}

pub(super) fn run_sonar_prefer_nullish_coalescing_assign(
    config: &RuleConfig,
    run: RuleRun<'_>,
) -> Vec<Violation> {
    run.check(
        "sonar/prefer-nullish-coalescing-assign",
        config,
        scan_prefer_nullish_coalescing_assign_from_tree,
        |record, content| {
            let replacement = format!("{} ??= {};", record.target, record.value);
            let message_replacement = format!("{} ??= {}", record.target, record.value);
            Report {
                line: record.line,
                message: format!(
                    "Prefer `{message_replacement}` over explicit nullish guard assignment — ??= only assigns when null or undefined."
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
