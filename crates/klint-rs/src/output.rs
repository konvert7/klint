use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Violation {
    pub file: String,
    pub line: usize,
    pub rule: String,
    pub message: String,
    pub severity: String,
    pub fix: Option<serde_json::Value>,
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
        }
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
