use super::{Report, RuleScan};
use crate::syntax::{SameFileBindings, date_equality_record, floating_promise_record};

const FLOATING_PROMISE_MESSAGE: &str =
    "This promise is neither awaited nor handled — `await` it, return it, or add `.catch()`.";

pub(super) fn run_no_date_equality(scan: RuleScan<'_>) -> Vec<Report> {
    if scan.index.has_syntax_error {
        return Vec::new();
    }

    let bindings = SameFileBindings::new(&scan.index.identifiers, scan.source);

    scan.index
        .binary_expressions
        .iter()
        .filter_map(|binary| date_equality_record(*binary, &bindings, scan.source))
        .map(|record| Report {
            line: record.line,
            message: format!(
                "Comparing two Date objects with `{}` compares identity, not time — compare `a.getTime()` values instead.",
                record.operator
            ),
            fix: None,
            level: None,
            bytes: None,
        })
        .collect()
}

pub(super) fn run_no_floating_promise(scan: RuleScan<'_>) -> Vec<Report> {
    if scan.index.has_syntax_error {
        return Vec::new();
    }

    let bindings = SameFileBindings::new(&scan.index.identifiers, scan.source);

    scan.index
        .expression_statements
        .iter()
        .filter_map(|statement| floating_promise_record(*statement, &bindings, scan.source))
        .map(|record| Report {
            line: record.line,
            message: FLOATING_PROMISE_MESSAGE.to_string(),
            fix: None,
            level: None,
            bytes: None,
        })
        .collect()
}
