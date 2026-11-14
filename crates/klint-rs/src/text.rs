use std::ffi::OsStr;
use std::time::Duration;

use klint_rs::{Span, Violation, fix_hint};

const HEADER_WIDTH: usize = 80;

pub struct TextReport {
    pub stdout: String,
    pub stderr: String,
}

const RED: &str = "31";
const YELLOW: &str = "33";
const DIM: &str = "2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextStyle {
    Hook,
    Plain,
    Colour,
}

impl TextStyle {
    fn paint(self, code: &str, text: &str) -> String {
        if self == TextStyle::Colour {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    fn shows_spans(self) -> bool {
        self != TextStyle::Hook
    }
}

pub fn style_for(hook: bool, terminal: bool, no_color: Option<&OsStr>) -> TextStyle {
    if hook {
        TextStyle::Hook
    } else if terminal && no_color.is_none_or(OsStr::is_empty) {
        TextStyle::Colour
    } else {
        TextStyle::Plain
    }
}

pub fn render(violations: &[Violation], elapsed: Duration, style: TextStyle) -> TextReport {
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
        stderr.push_str(&section("warning(s)", &warnings, YELLOW, style));
    }

    if !errors.is_empty() {
        stderr.push_str(&section("error(s)", &errors, RED, style));
    }

    if let Some(hint) = fix_hint(violations) {
        stderr.push_str(&format!(
            "\n{}\n",
            style.paint(DIM, &format!("klint: {hint}"))
        ));
    }

    let finished = format!("klint: finished in {duration}");
    stderr.push_str(&format!("\n{}\n", style.paint(DIM, &finished)));
    TextReport {
        stdout: String::new(),
        stderr,
    }
}

pub fn warnings_section(violations: &[Violation], style: TextStyle) -> String {
    section(
        "warning(s)",
        &with_severity(violations, "warn"),
        YELLOW,
        style,
    )
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

fn section(label: &str, violations: &[&Violation], tone: &str, style: TextStyle) -> String {
    let blocks: Vec<String> = violations
        .iter()
        .map(|violation| block(violation, tone, style))
        .collect();
    let count = format!("{} {label}", violations.len());
    format!(
        "klint: {}\n\n{}",
        style.paint(tone, &count),
        blocks.join("\n")
    )
}

fn block(violation: &Violation, tone: &str, style: TextStyle) -> String {
    let marker = if violation.severity == "warn" {
        "⚠"
    } else {
        "×"
    };
    let span = violation.span.as_ref().filter(|_| style.shows_spans());
    let location = match span {
        Some(span) => format!("{}:{}:{}", violation.file, violation.line, span.column),
        None => format!("{}:{}", violation.file, violation.line),
    };
    let header = format!("{location}  [{}]", violation.rule);
    let rule_line = "━".repeat(HEADER_WIDTH.saturating_sub(utf16_len(&header)));
    let mut text = format!(
        "{}  [{}] {}\n\n  {} {}\n",
        style.paint(DIM, &location),
        violation.rule,
        style.paint(DIM, &rule_line),
        style.paint(tone, marker),
        violation.message
    );

    if let Some(span) = span {
        text.push_str(&code_frame(span, violation.line, tone, style));
    }

    text
}

fn code_frame(span: &Span, line: usize, tone: &str, style: TextStyle) -> String {
    let number = line.to_string();
    let blank_gutter = format!("  {} │", " ".repeat(number.len()));
    let carets = "^".repeat(span.width);
    format!(
        "\n  {} {}\n  {} {}{}\n",
        style.paint(DIM, &format!("> {number} │")),
        span.source_line,
        style.paint(DIM, &blank_gutter),
        whitespace_before(span),
        style.paint(tone, &carets)
    )
}

fn whitespace_before(span: &Span) -> String {
    span.source_line
        .chars()
        .take(span.column.saturating_sub(1))
        .map(|character| if character == '\t' { '\t' } else { ' ' })
        .collect()
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
            span: None,
        }
    }

    fn spanned(severity: &str) -> Violation {
        Violation {
            line: 3,
            span: Some(Span {
                source_line: "const m = s.match(/x/);".to_string(),
                column: 11,
                width: 12,
            }),
            ..violation("src/a.ts", severity)
        }
    }

    fn stderr_of(violations: &[Violation], style: TextStyle) -> String {
        render(violations, Duration::from_millis(5), style).stderr
    }

    #[test]
    fn header_rule_line_never_goes_negative_on_long_paths() {
        let long = "a/".repeat(60);
        let stderr = stderr_of(&[violation(&long, "error")], TextStyle::Plain);
        assert!(stderr.contains(&format!("{long}:1  [r] \n")));
    }

    #[test]
    fn report_points_at_fix_when_a_violation_is_fixable() {
        let mut fixable = violation("a.ts", "error");
        fixable.fix = Some(serde_json::json!({}));
        assert!(stderr_of(&[fixable], TextStyle::Plain).contains("run `klint --fix`"));
    }

    #[test]
    fn report_stays_silent_about_fix_when_nothing_is_fixable() {
        assert!(!stderr_of(&[violation("a.ts", "error")], TextStyle::Plain).contains("--fix"));
    }

    #[test]
    fn colour_paints_errors_red_and_warnings_yellow() {
        let stderr = stderr_of(&[spanned("error"), spanned("warn")], TextStyle::Colour);
        assert!(stderr.contains("\x1b[31m×\x1b[0m"), "{stderr:?}");
        assert!(stderr.contains("\x1b[33m⚠\x1b[0m"), "{stderr:?}");
        assert!(stderr.contains("\x1b[31m^^^^^^^^^^^^\x1b[0m"), "{stderr:?}");
        assert!(stderr.contains("\x1b[33m^^^^^^^^^^^^\x1b[0m"), "{stderr:?}");
    }

    #[test]
    fn colour_dims_file_paths_and_hints() {
        let mut fixable = spanned("error");
        fixable.fix = Some(serde_json::json!({}));
        let stderr = stderr_of(&[fixable], TextStyle::Colour);
        assert!(stderr.contains("\x1b[2msrc/a.ts:3:11\x1b[0m"), "{stderr:?}");
        assert!(
            stderr.contains("\x1b[2mklint: 1 violation(s) can be fixed"),
            "{stderr:?}"
        );
        assert!(stderr.contains("\x1b[2mklint: finished in"), "{stderr:?}");
    }

    #[test]
    fn plain_style_has_no_escape_codes() {
        let stderr = stderr_of(&[spanned("error"), spanned("warn")], TextStyle::Plain);
        assert!(!stderr.contains('\x1b'), "{stderr:?}");
        assert!(stderr.contains("^^^^"), "{stderr:?}");
    }

    #[test]
    fn carets_sit_under_the_reported_span() {
        let stderr = stderr_of(&[spanned("error")], TextStyle::Plain);
        let frame = "  > 3 │ const m = s.match(/x/);\n      │           ^^^^^^^^^^^^\n";
        assert!(stderr.contains(frame), "{stderr}");
        assert!(stderr.contains("src/a.ts:3:11  [r] "), "{stderr}");
    }

    #[test]
    fn carets_keep_tabs_so_they_line_up_with_indented_source() {
        let mut tabbed = spanned("error");
        tabbed.span = Some(Span {
            source_line: "\tlet x = y;".to_string(),
            column: 6,
            width: 1,
        });
        let stderr = stderr_of(&[tabbed], TextStyle::Plain);
        assert!(stderr.contains("      │ \t    ^\n"), "{stderr:?}");
    }

    #[test]
    fn line_only_violations_keep_todays_block() {
        let stderr = stderr_of(&[violation("a.ts", "error")], TextStyle::Plain);
        assert!(stderr.contains("a.ts:1  [r] "), "{stderr}");
        assert!(!stderr.contains('│'), "{stderr}");
    }

    #[test]
    fn hook_style_prints_todays_output_byte_for_byte() {
        let stderr = stderr_of(&[spanned("error")], TextStyle::Hook);
        let header = "src/a.ts:3  [r]";
        let expected = format!(
            "klint: 1 error(s)\n\n{header} {}\n\n  × m\n\nklint: finished in 5ms\n",
            "━".repeat(HEADER_WIDTH - header.len())
        );
        assert_eq!(stderr, expected);
    }

    #[test]
    fn colour_needs_a_terminal_and_no_no_color() {
        assert_eq!(style_for(false, true, None), TextStyle::Colour);
        assert_eq!(
            style_for(false, true, Some(OsStr::new(""))),
            TextStyle::Colour
        );
        assert_eq!(
            style_for(false, true, Some(OsStr::new("1"))),
            TextStyle::Plain
        );
        assert_eq!(style_for(false, false, None), TextStyle::Plain);
    }

    #[test]
    fn hook_runs_keep_todays_output_even_on_a_terminal() {
        assert_eq!(style_for(true, true, None), TextStyle::Hook);
    }

    #[test]
    fn durations_match_the_ts_engine_format() {
        assert_eq!(format_duration(Duration::from_micros(4_600)), "5ms");
        assert_eq!(format_duration(Duration::from_millis(1_234)), "1.23s");
        assert_eq!(format_duration(Duration::from_millis(12_345)), "12.3s");
    }
}
