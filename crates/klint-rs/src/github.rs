use klint_rs::Violation;

pub fn render(violations: &[Violation]) -> String {
    violations.iter().map(command).collect()
}

fn command(violation: &Violation) -> String {
    format!(
        "::{} file={},line={},title={}::{}\n",
        level(&violation.severity),
        escape_property(&violation.file),
        violation.line,
        escape_property(&violation.rule),
        escape_data(&violation.message),
    )
}

fn level(severity: &str) -> &'static str {
    match severity {
        "warn" => "warning",
        _ => "error",
    }
}

// GitHub workflow commands: data escapes % CR LF; property values also escape : and ,
// (actions/toolkit packages/core/src/command.ts).
fn escape_data(text: &str) -> String {
    text.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

fn escape_property(text: &str) -> String {
    escape_data(text).replace(':', "%3A").replace(',', "%2C")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn violation(file: &str, severity: &str, message: &str) -> Violation {
        Violation {
            file: file.to_string(),
            line: 3,
            rule: "arch/imports".to_string(),
            message: message.to_string(),
            severity: severity.to_string(),
            fix: None,
            span: None,
        }
    }

    #[test]
    fn error_becomes_an_error_annotation() {
        assert_eq!(
            render(&[violation("src/a.ts", "error", "Use logger")]),
            "::error file=src/a.ts,line=3,title=arch/imports::Use logger\n"
        );
    }

    #[test]
    fn warn_becomes_a_warning_annotation() {
        assert!(render(&[violation("src/a.ts", "warn", "m")]).starts_with("::warning "));
    }

    #[test]
    fn message_newlines_and_percent_are_escaped() {
        assert_eq!(
            render(&[violation("a.ts", "error", "100% wrong\r\nsecond line")]),
            "::error file=a.ts,line=3,title=arch/imports::100%25 wrong%0D%0Asecond line\n"
        );
    }

    #[test]
    fn file_property_escapes_separators_and_keeps_backslashes() {
        assert_eq!(
            render(&[violation(r"src\a,b:c%.ts", "error", "m")]),
            "::error file=src\\a%2Cb%3Ac%25.ts,line=3,title=arch/imports::m\n"
        );
    }

    #[test]
    fn no_violations_print_nothing() {
        assert_eq!(render(&[]), "");
    }
}
