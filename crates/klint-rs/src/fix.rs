use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::output::Violation;

#[derive(Debug, PartialEq, Eq)]
struct LineFix {
    start_line: usize,
    end_line: usize,
    replacement: String,
}

pub(crate) fn has_fixes(violations: &[Violation]) -> bool {
    violations
        .iter()
        .any(|violation| line_fix(violation).is_some())
}

pub(crate) fn apply_fixes(violations: &[Violation], root: &Path) -> Result<usize, String> {
    let mut by_file: BTreeMap<&str, Vec<LineFix>> = BTreeMap::new();
    for violation in violations {
        if let Some(fix) = line_fix(violation) {
            by_file.entry(&violation.file).or_default().push(fix);
        }
    }

    let mut applied = 0;
    for (file, fixes) in by_file {
        applied += apply_file_fixes(&root.join(file), fixes)?;
    }
    Ok(applied)
}

fn line_fix(violation: &Violation) -> Option<LineFix> {
    let fix = violation.fix.as_ref()?;
    let line = |key: &str| {
        fix.get(key)
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .filter(|&value| value > 0)
    };
    let start_line = line("startLine")?;
    let end_line = line("endLine")?.max(start_line);
    let replacement = fix.get("replacement")?.as_str()?.to_string();
    Some(LineFix {
        start_line,
        end_line,
        replacement,
    })
}

fn apply_file_fixes(path: &Path, fixes: Vec<LineFix>) -> Result<usize, String> {
    let content = fs::read_to_string(path)
        .map_err(|err| format!("klint-rs: failed to read {}: {err}", path.display()))?;
    let mut lines: Vec<String> = content.split('\n').map(str::to_string).collect();

    let mut applied_ranges: Vec<(usize, usize)> = Vec::new();
    for fix in largest_range_first(fixes) {
        let overlaps_applied = applied_ranges
            .iter()
            .any(|&(start, end)| fix.start_line <= end && fix.end_line >= start);
        if overlaps_applied || fix.end_line > lines.len() {
            continue;
        }
        lines.splice(
            fix.start_line - 1..fix.end_line,
            fix.replacement.split('\n').map(str::to_string),
        );
        applied_ranges.push((fix.start_line, fix.end_line));
    }

    if !applied_ranges.is_empty() {
        fs::write(path, lines.join("\n"))
            .map_err(|err| format!("klint-rs: failed to write {}: {err}", path.display()))?;
    }
    Ok(applied_ranges.len())
}

fn largest_range_first(mut fixes: Vec<LineFix>) -> Vec<LineFix> {
    fixes.sort_by_key(|fix| std::cmp::Reverse(fix.end_line));
    fixes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fix(start_line: usize, end_line: usize, replacement: &str) -> LineFix {
        LineFix {
            start_line,
            end_line,
            replacement: replacement.to_string(),
        }
    }

    fn apply_to(content: &str, fixes: Vec<LineFix>) -> (usize, String) {
        let dir = std::env::temp_dir().join(format!(
            "klint-rs-fix-unit-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("create dir");
        let path = dir.join("a.ts");
        fs::write(&path, content).expect("write");
        let applied = apply_file_fixes(&path, fixes).expect("apply");
        let result = fs::read_to_string(&path).expect("read");
        let _ = fs::remove_dir_all(dir);
        (applied, result)
    }

    #[test]
    fn skips_a_fix_that_overlaps_one_already_applied() {
        let (applied, result) = apply_to(
            "a\nb\nc\nd\n",
            vec![fix(2, 2, "B1"), fix(1, 3, "ABC"), fix(4, 4, "D")],
        );
        assert_eq!(applied, 2);
        assert_eq!(result, "ABC\nD\n");
    }

    #[test]
    fn multi_line_replacement_keeps_lines_above_in_place() {
        let (applied, result) = apply_to("a\nb\nc", vec![fix(1, 1, "a1\na2"), fix(3, 3, "C")]);
        assert_eq!(applied, 2);
        assert_eq!(result, "a1\na2\nb\nC");
    }

    #[test]
    fn ignores_a_fix_past_the_end_of_the_file() {
        let (applied, result) = apply_to("a\n", vec![fix(9, 9, "x")]);
        assert_eq!(applied, 0);
        assert_eq!(result, "a\n");
    }
}
