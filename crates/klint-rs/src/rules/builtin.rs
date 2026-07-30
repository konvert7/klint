use super::{Report, RuleScan, line_fix};
use crate::syntax::{
    find_nested_template_literals, is_json_parse_call, scan_statement_run, string_match_record,
    sync_call_name,
};

pub(super) fn run_no_string_match(scan: RuleScan<'_>) -> Vec<Report> {
    scan.index
        .calls
        .iter()
        .filter_map(|call| string_match_record(call.node, scan.source))
        .map(|record| {
            let replacement = format!("new RegExp({}).exec({})", record.regex, record.receiver);
            Report {
                line: record.line,
                message: format!(
                    "Use RegExp.exec() instead of String.match() for non-global regexes — use {replacement} instead."
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

pub(super) fn run_no_nested_template_literals(scan: RuleScan<'_>) -> Vec<Report> {
    let mut records = Vec::new();
    for template in &scan.index.templates {
        let mut cursor = template.walk();
        for child in template.children(&mut cursor) {
            if child.kind() == "template_substitution" {
                find_nested_template_literals(child, &mut records);
            }
        }
    }

    records
        .into_iter()
        .map(|record| Report {
            line: record.line,
            message:
                "Nested template literal — extract the inner template to a variable to improve readability."
                    .to_string(),
            fix: None,
        })
        .collect()
}

pub(super) fn run_no_consecutive_array_push(scan: RuleScan<'_>) -> Vec<Report> {
    let mut records = Vec::new();
    for container in &scan.index.containers {
        scan_statement_run(*container, scan.source, &mut records);
    }

    records
        .into_iter()
        .map(|record| Report {
            line: record.line,
            message: format!(
                "{} consecutive .push() calls on `{}` — combine into a single .push(a, b, …) call.",
                record.count, record.receiver
            ),
            fix: None,
        })
        .collect()
}

pub(super) fn run_no_unguarded_json_parse(scan: RuleScan<'_>) -> Vec<Report> {
    scan.index
        .calls
        .iter()
        .filter(|call| !call.inside_try && is_json_parse_call(call.node, scan.source))
        .map(|call| Report {
            line: call.node.start_position().row + 1,
            message:
                "JSON.parse() called without a surrounding try/catch — a malformed payload will throw an unhandled exception."
                    .to_string(),
            fix: None,
        })
        .collect()
}

pub(super) fn run_no_sync_in_async(scan: RuleScan<'_>) -> Vec<Report> {
    scan.index
        .calls
        .iter()
        .filter(|call| call.in_async)
        .filter_map(|call| sync_call_name(call.node, scan.source).map(|name| (call.node, name)))
        .map(|(node, name)| Report {
            line: node.start_position().row + 1,
            message: format!(
                "{name}() blocks the event loop inside an async function — use the async equivalent from node:fs/promises."
            ),
            fix: None,
        })
        .collect()
}
