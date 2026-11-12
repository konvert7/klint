use std::time::Duration;

use klint_rs::Violation;

const HEADER_WIDTH: usize = 80;

pub struct TextReport {
    pub stdout: String,
    pub stderr: String,
}

pub fn render(violations: &[Violation], elapsed: Duration) -> TextReport {
    let warnings: Vec<&Violation> = with_severity(violations, "warn");
    let errors: Vec<&Violation> = with_severity(violations, "error");
    let duration = format_duration(elapsed);

    if warnings.is_empty() && errors.is_empty() {
        return TextReport {
            stdout: format!("klint: 0 violations in {duration}\n"),
            stderr: String::new(),
        };
    }

    let mut stderr = String::new();
    if !warnings.is_empty() {
        stderr.push_str(&section("warning(s)", &warnings));
    }
    if !errors.is_empty() {
        stderr.push_str(&section("error(s)", &errors));
    }
    stderr.push_str(&format!("\nklint: finished in {duration}\n"));
    TextReport {
        stdout: String::new(),
        stderr,
    }
}

pub fn warnings_section(violations: &[Violation]) -> String {
    section("warning(s)", &with_severity(violations, "warn"))
}

pub fn format_duration(elapsed: Duration) -> String {
    let millis = elapsed.as_secs_f64() * 1000.0;
    if millis < 1000.0 {
        format!("{}ms", millis.round())
    } else if millis < 10_000.0 {
        format!("{:.2}s", millis / 1000.0)
    } else {
        format!("{:.1}s", millis / 1000.0)
    }
}

fn with_severity<'a>(violations: &'a [Violation], severity: &str) -> Vec<&'a Violation> {
    violations
        .iter()
        .filter(|violation| violation.severity == severity)
        .collect()
}

fn section(label: &str, violations: &[&Violation]) -> String {
    let blocks: Vec<String> = violations
        .iter()
        .map(|violation| block(violation))
        .collect();
    format!(
        "klint: {} {label}\n\n{}",
        violations.len(),
        blocks.join("\n")
    )
}

fn block(violation: &Violation) -> String {
    let marker = if violation.severity == "warn" {
        "⚠"
    } else {
        "×"
    };
    let header = format!(
        "{}:{}  [{}]",
        violation.file, violation.line, violation.rule
    );
    let rule_line = "━".repeat(HEADER_WIDTH.saturating_sub(utf16_len(&header)));
    format!("{header} {rule_line}\n\n  {marker} {}\n", violation.message)
}

fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn violation(file: &str, severity: &str) -> Violation {
        Violation {
            file: file.to_string(),
            line: 1,
            rule: "r".to_string(),
            message: "m".to_string(),
            severity: severity.to_string(),
            fix: None,
        }
    }

    #[test]
    fn header_rule_line_never_goes_negative_on_long_paths() {
        let long = "a/".repeat(60);
        let report = render(&[violation(&long, "error")], Duration::from_millis(5));
        assert!(report.stderr.contains(&format!("{long}:1  [r] \n")));
    }

    #[test]
    fn durations_match_the_ts_engine_format() {
        assert_eq!(format_duration(Duration::from_micros(4_600)), "5ms");
        assert_eq!(format_duration(Duration::from_millis(1_234)), "1.23s");
        assert_eq!(format_duration(Duration::from_millis(12_345)), "12.3s");
    }
}
