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
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct JsonOutput {
    pub violations: Vec<Violation>,
    pub summary: Summary,
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
    };

    JsonOutput {
        violations,
        summary,
    }
}
