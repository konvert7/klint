use super::{Report, RuleScan, line_fix};
use crate::syntax::{
    prefer_at_record, prefer_nullish_coalescing_assign_record, prefer_string_raw_record,
    prefer_string_raw_regexp_record, prefer_string_replaceall_record, single_char_class_record,
};

pub(super) fn run_sonar_no_single_char_class(scan: RuleScan<'_>) -> Vec<Report> {
    scan.index
        .regexes
        .iter()
        .filter_map(|node| single_char_class_record(*node, scan.source))
        .map(|record| Report {
            line: record.line,
            message: format!(
                "Character class [{}] contains a single element — remove the brackets.",
                record.class
            ),
            fix: line_fix(
                scan.content,
                record.start_row,
                record.end_row,
                record.start_byte,
                record.end_byte,
                &record.fixed_regex,
            ),
        })
        .collect()
}

pub(super) fn run_sonar_prefer_at(scan: RuleScan<'_>) -> Vec<Report> {
    scan.index
        .subscripts
        .iter()
        .filter_map(|node| prefer_at_record(*node, scan.source))
        .map(|record| {
            let replacement = format!("{}.at(-{})", record.base, record.offset);
            Report {
                line: record.line,
                message: format!(
                    "Prefer {} over {}[{}.length - {}] for cleaner negative indexing.",
                    replacement, record.base, record.base, record.offset
                ),
                fix: line_fix(
                    scan.content,
                    record.start_row,
                    record.end_row,
                    record.start_byte,
                    record.end_byte,
                    &replacement,
                ),
            }
        })
        .collect()
}

pub(super) fn run_sonar_prefer_string_replaceall(scan: RuleScan<'_>) -> Vec<Report> {
    scan.index
        .calls
        .iter()
        .filter_map(|call| prefer_string_replaceall_record(call.node, scan.source))
        .map(|record| {
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
                    scan.content,
                    record.start_row,
                    record.end_row,
                    record.start_byte,
                    record.end_byte,
                    &replacement,
                ),
            }
        })
        .collect()
}

pub(super) fn run_sonar_prefer_string_raw_regexp(scan: RuleScan<'_>) -> Vec<Report> {
    scan.index
        .new_expressions
        .iter()
        .filter_map(|node| prefer_string_raw_regexp_record(*node, scan.source))
        .map(|record| Report {
            line: record.line,
            message:
                "Use String.raw`...` for RegExp template argument to avoid double backslashes (Sonar S7780)."
                    .to_string(),
            fix: line_fix(
                scan.content,
                record.start_row,
                record.end_row,
                record.start_byte,
                record.end_byte,
                &record.fixed_arg,
            ),
        })
        .collect()
}

pub(super) fn run_sonar_prefer_string_raw(scan: RuleScan<'_>) -> Vec<Report> {
    scan.index
        .strings
        .iter()
        .filter_map(|node| prefer_string_raw_record(*node, scan.source))
        .map(|record| Report {
            line: record.line,
            message:
                "String literal with escaped backslashes — use String.raw`...` for clarity (Sonar S6535)."
                    .to_string(),
            fix: line_fix(
                scan.content,
                record.start_row,
                record.end_row,
                record.start_byte,
                record.end_byte,
                &record.fixed,
            ),
        })
        .collect()
}

pub(super) fn run_sonar_prefer_nullish_coalescing_assign(scan: RuleScan<'_>) -> Vec<Report> {
    scan.index
        .if_statements
        .iter()
        .filter_map(|node| prefer_nullish_coalescing_assign_record(*node, scan.source))
        .map(|record| {
            let replacement = format!("{} ??= {};", record.target, record.value);
            let message_replacement = format!("{} ??= {}", record.target, record.value);
            Report {
                line: record.line,
                message: format!(
                    "Prefer `{message_replacement}` over explicit nullish guard assignment — ??= only assigns when null or undefined."
                ),
                fix: line_fix(
                    scan.content,
                    record.start_row,
                    record.end_row,
                    record.start_byte,
                    record.end_byte,
                    &replacement,
                ),
            }
        })
        .collect()
}
