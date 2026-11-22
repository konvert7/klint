use std::ops::Range;

use super::{Report, RuleScan, line_end_byte, line_start_byte};
use crate::syntax::{NamingFinding, naming_finding};

pub(super) fn run_naming_convention(scan: RuleScan<'_>) -> Vec<Report> {
    if scan.index.has_syntax_error {
        return Vec::new();
    }

    scan.index
        .declarations
        .iter()
        .filter_map(|node| naming_finding(*node, scan.source, scan.language))
        .map(|finding| report(finding, scan.content))
        .collect()
}

fn report(finding: NamingFinding<'_>, content: &str) -> Report {
    let fix = finding
        .rename
        .as_ref()
        .and_then(|rename| rename_fix(content, &rename.edits));

    Report {
        line: finding.name.start_position().row + 1,
        message: finding.message,
        fix,
        level: None,
        bytes: Some(finding.name.byte_range()),
    }
}

fn rename_fix(content: &str, edits: &[(Range<usize>, String)]) -> Option<serde_json::Value> {
    let (first, _) = edits.first()?;
    let (last, _) = edits.last()?;
    let lines_start = line_start_byte(content, first.start);
    let lines_end = line_end_byte(content, last.end);
    let mut lines = content.get(lines_start..lines_end)?.to_string();

    for (range, text) in edits.iter().rev() {
        lines.replace_range(range.start - lines_start..range.end - lines_start, text);
    }

    let start_line = content.get(..lines_start)?.matches('\n').count() + 1;
    let end_line = start_line + content.get(lines_start..lines_end)?.matches('\n').count();

    Some(serde_json::json!({
        "startLine": start_line,
        "endLine": end_line,
        "replacement": lines,
    }))
}
