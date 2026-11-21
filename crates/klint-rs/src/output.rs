use std::ops::Range;

use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Violation {
    pub file: String,
    pub line: usize,
    pub rule: String,
    pub message: String,
    pub severity: String,
    pub fix: Option<serde_json::Value>,
    #[serde(flatten)]
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub source_line: String,
    pub column: usize,
    pub width: usize,
}

impl Span {
    pub(crate) fn within(content: &str, bytes: Range<usize>) -> Option<Span> {
        let start = bytes.start;
        let line_start = content
            .get(..start)?
            .rfind('\n')
            .map_or(0, |index| index + 1);
        let line_end = content[start..]
            .find('\n')
            .map_or(content.len(), |index| start + index);
        let end = bytes.end.clamp(start, line_end);
        Some(Span {
            source_line: content[line_start..line_end]
                .trim_end_matches('\r')
                .to_string(),
            column: content.get(line_start..start)?.chars().count() + 1,
            width: content.get(start..end)?.chars().count().max(1),
        })
    }
}

impl Span {
    fn utf16_columns(&self) -> (usize, usize) {
        let mut chars = self.source_line.chars();
        let before: usize = chars
            .by_ref()
            .take(self.column - 1)
            .map(char::len_utf16)
            .sum();
        let inside: usize = chars.take(self.width).map(char::len_utf16).sum();

        (before + 1, before + inside.max(1) + 1)
    }
}

impl Serialize for Span {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let (column, end_column) = self.utf16_columns();
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("column", &column)?;
        map.serialize_entry("endColumn", &end_column)?;
        map.end()
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Summary {
    pub errors: usize,
    pub warnings: usize,
    #[serde(skip_serializing_if = "is_zero")]
    pub fixable: usize,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct JsonOutput {
    pub violations: Vec<Violation>,
    pub summary: Summary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

fn is_zero(count: &usize) -> bool {
    *count == 0
}

pub fn fixable_count(violations: &[Violation]) -> usize {
    violations
        .iter()
        .filter(|violation| violation.fix.is_some())
        .count()
}

pub fn fix_hint(violations: &[Violation]) -> Option<String> {
    match fixable_count(violations) {
        0 => None,
        fixable => Some(format!(
            "{fixable} violation(s) can be fixed automatically: run `klint --fix`."
        )),
    }
}

pub(crate) fn output_from_violations(violations: Vec<Violation>) -> JsonOutput {
    let summary = Summary {
        errors: violations
            .iter()
            .filter(|violation| violation.severity == "error")
            .count(),
        warnings: violations
            .iter()
            .filter(|violation| violation.severity == "warn")
            .count(),
        fixable: fixable_count(&violations),
    };
    let hint = fix_hint(&violations);

    JsonOutput {
        violations,
        summary,
        hint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn violation(fix: Option<serde_json::Value>) -> Violation {
        Violation {
            file: "a.ts".to_string(),
            line: 1,
            rule: "r".to_string(),
            message: "m".to_string(),
            severity: "error".to_string(),
            fix,
            span: None,
        }
    }

    fn spanned(line: &str, needle: &str) -> Violation {
        let start = line.find(needle).expect("needle is in the line");
        Violation {
            span: Span::within(line, start..start + needle.len()),
            ..violation(None)
        }
    }

    #[test]
    fn json_has_no_column_when_the_rule_knows_none() {
        let json = serde_json::to_value(violation(None)).expect("serializes");
        assert!(json.get("column").is_none());
        assert!(json.get("endColumn").is_none());
    }

    #[test]
    fn json_columns_are_one_based_and_end_exclusive() {
        let json = serde_json::to_value(spanned("let x = JSON.parse(s);", "JSON.parse"))
            .expect("serializes");
        assert_eq!(
            (json["column"].clone(), json["endColumn"].clone()),
            (9.into(), 19.into())
        );
    }

    #[test]
    fn json_columns_count_utf16_units_like_editors_do() {
        let json = serde_json::to_value(spanned("const s = \"😀é\"; JSON.parse(s);", "JSON.parse"))
            .expect("serializes");
        assert_eq!(
            (json["column"].clone(), json["endColumn"].clone()),
            (18.into(), 28.into())
        );
    }

    #[test]
    fn hint_counts_only_violations_with_a_fix() {
        let violations = [violation(Some(serde_json::json!({}))), violation(None)];
        assert_eq!(
            fix_hint(&violations).expect("one violation is fixable"),
            "1 violation(s) can be fixed automatically: run `klint --fix`."
        );
    }

    #[test]
    fn json_keeps_its_old_shape_when_nothing_is_fixable() {
        let json = serde_json::to_value(output_from_violations(vec![violation(None)]))
            .expect("output serializes");
        assert!(json.get("hint").is_none());
        assert!(json["summary"].get("fixable").is_none());
    }

    #[test]
    fn json_tells_the_reader_to_run_fix() {
        let fixable = violation(Some(serde_json::json!({})));
        let json =
            serde_json::to_value(output_from_violations(vec![fixable])).expect("output serializes");
        assert_eq!(json["summary"]["fixable"], 1);
        assert!(
            json["hint"]
                .as_str()
                .expect("hint is set")
                .contains("klint --fix")
        );
    }
}
