mod options;
mod schema_shapes;

pub(crate) use options::BlankLineOptions;
pub(super) use options::blank_line_options;
pub(super) use schema_shapes::{after_schema, before_schema};

use super::{Report, RuleOptions, RuleScan};
use crate::syntax::{BlankLineFinding, missing_blank_after, missing_blank_before};
use serde_json::json;

pub(super) fn run_blank_line_before_control_flow(scan: RuleScan<'_>) -> Vec<Report> {
    let RuleOptions::BlankLines(options) = scan.options else {
        return Vec::new();
    };
    if scan.index.has_syntax_error {
        return Vec::new();
    }
    let findings = missing_blank_before(
        &scan.index.statement_blocks,
        scan.content,
        scan.language,
        options.statements,
        options.allow_setup,
    );
    reports(findings, scan.content, before_message)
}

pub(super) fn run_blank_line_after_block(scan: RuleScan<'_>) -> Vec<Report> {
    let RuleOptions::BlankLines(options) = scan.options else {
        return Vec::new();
    };
    if scan.index.has_syntax_error {
        return Vec::new();
    }
    let findings = missing_blank_after(
        &scan.index.statement_blocks,
        scan.content,
        scan.language,
        options.statements,
    );
    reports(findings, scan.content, after_message)
}

fn reports(
    findings: Vec<BlankLineFinding>,
    content: &str,
    message: fn(&str) -> String,
) -> Vec<Report> {
    findings
        .into_iter()
        .map(|finding| Report {
            line: finding.line,
            message: message(finding.keyword),
            fix: insert_blank_line_fix(content, finding.insertion_line),
        })
        .collect()
}

fn insert_blank_line_fix(content: &str, insertion_line: usize) -> Option<serde_json::Value> {
    let lines: Vec<&str> = content.split('\n').collect();
    if insertion_line < 2 || insertion_line > lines.len() {
        return None;
    }
    let text = lines[insertion_line - 1];
    let above = lines[insertion_line - 2];
    let empty = if above.ends_with('\r') { "\r" } else { "" };
    Some(json!({
        "startLine": insertion_line,
        "endLine": insertion_line,
        "replacement": format!("{empty}\n{text}"),
    }))
}

fn before_message(keyword: &str) -> String {
    format!("Missing blank line before `{keyword}` — separate it from the statement above.")
}

fn after_message(keyword: &str) -> String {
    format!("Missing blank line after `{keyword}` block — separate it from the statement below.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn lf_file_gets_an_lf_blank_line_above_the_statement() {
        assert_eq!(
            insert_blank_line_fix("foo();\nif (a) {\n}\n", 2),
            Some(json!({"startLine": 2, "endLine": 2, "replacement": "\nif (a) {"}))
        );
    }

    #[test]
    fn crlf_file_gets_a_crlf_blank_line_above_the_statement() {
        assert_eq!(
            insert_blank_line_fix("foo();\r\nif (a) {\r\n}\r\n", 2),
            Some(json!({"startLine": 2, "endLine": 2, "replacement": "\r\nif (a) {\r"}))
        );
    }

    #[test]
    fn mixed_file_copies_the_ending_of_the_line_above() {
        assert_eq!(
            insert_blank_line_fix("foo();\nif (a) {\r\n}\r\n", 2),
            Some(json!({"startLine": 2, "endLine": 2, "replacement": "\nif (a) {\r"}))
        );
    }

    #[test]
    fn insertion_on_the_first_line_is_refused() {
        assert_eq!(insert_blank_line_fix("a\n", 1), None);
    }

    #[test]
    fn insertion_past_the_last_line_is_refused() {
        assert_eq!(insert_blank_line_fix("a\nb", 5), None);
    }

    #[test]
    fn before_message_names_the_keyword() {
        assert_eq!(
            before_message("if"),
            "Missing blank line before `if` — separate it from the statement above."
        );
    }

    #[test]
    fn after_message_names_the_keyword() {
        assert_eq!(
            after_message("if"),
            "Missing blank line after `if` block — separate it from the statement below."
        );
    }
}
