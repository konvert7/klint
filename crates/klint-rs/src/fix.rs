use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::output::Violation;
use crate::rules::has_safe_fix;

#[derive(Debug, PartialEq, Eq)]
struct LineFix {
    start_line: usize,
    end_line: usize,
    replacement: String,
}

pub const MAX_FIX_PASSES: usize = 10;

pub(crate) struct FixPasses<T> {
    pub(crate) applied: usize,
    pub(crate) last: T,
    pub(crate) converged: bool,
}

pub(crate) fn fix_until_stable<T>(
    mut lint: impl FnMut() -> Result<T, String>,
    mut apply: impl FnMut(&T) -> Result<usize, String>,
    has_fixes: impl Fn(&T) -> bool,
) -> Result<FixPasses<T>, String> {
    let mut last = lint()?;
    let mut applied = 0;

    for _ in 0..MAX_FIX_PASSES {
        let applied_this_pass = apply(&last)?;
        applied += applied_this_pass;

        if applied_this_pass == 0 {
            return Ok(FixPasses {
                applied,
                last,
                converged: true,
            });
        }

        last = lint()?;
        if !has_fixes(&last) {
            return Ok(FixPasses {
                applied,
                last,
                converged: true,
            });
        }
    }

    Ok(FixPasses {
        applied,
        last,
        converged: false,
    })
}

#[derive(Default)]
pub(crate) struct Originals(BTreeMap<PathBuf, String>);

impl Originals {
    fn write(&mut self, path: &Path, before: &str, after: &str) -> Result<(), String> {
        fs::write(path, after)
            .map_err(|err| format!("klint: failed to write {}: {err}", path.display()))?;
        self.0
            .entry(path.to_path_buf())
            .or_insert_with(|| before.to_string());
        Ok(())
    }

    pub(crate) fn restore_after(self, error: String) -> String {
        let mut message = error;
        let mut restored = 0;

        for (path, original) in self.0 {
            match fs::write(&path, original) {
                Ok(()) => restored += 1,
                Err(err) => message.push_str(&format!(
                    "\nklint: failed to restore {}: {err}",
                    path.display()
                )),
            }
        }

        if restored > 0 {
            message.push_str(&format!(
                "\nklint: restored {restored} file(s) to their state before --fix"
            ));
        }

        message
    }
}

#[derive(Debug, Default)]
pub struct FixScope {
    pub safe_only: bool,
    pub paths: Option<Vec<PathBuf>>,
}

impl FixScope {
    fn selects(&self, violation: &Violation, root: &Path) -> bool {
        (!self.safe_only || has_safe_fix(&violation.rule)) && self.covers(root, &violation.file)
    }

    fn covers(&self, root: &Path, file: &str) -> bool {
        let Some(paths) = &self.paths else {
            return true;
        };
        let Ok(target) = fs::canonicalize(root.join(file)) else {
            return false;
        };

        paths.iter().any(|path| target.starts_with(path))
    }
}

pub(crate) fn has_fixes(violations: &[Violation], root: &Path, scope: &FixScope) -> bool {
    violations
        .iter()
        .any(|violation| scope.selects(violation, root) && line_fix(violation).is_some())
}

pub(crate) fn apply_fixes(
    violations: &[Violation],
    root: &Path,
    scope: &FixScope,
    originals: &mut Originals,
) -> Result<usize, String> {
    let mut by_file: BTreeMap<&str, Vec<LineFix>> = BTreeMap::new();

    for violation in violations.iter().filter(|v| scope.selects(v, root)) {
        if let Some(fix) = line_fix(violation) {
            by_file.entry(&violation.file).or_default().push(fix);
        }
    }

    let mut applied = 0;

    for (file, fixes) in by_file {
        applied += apply_file_fixes(&root.join(file), fixes, originals)?;
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

fn apply_file_fixes(
    path: &Path,
    fixes: Vec<LineFix>,
    originals: &mut Originals,
) -> Result<usize, String> {
    let content = fs::read_to_string(path)
        .map_err(|err| format!("klint: failed to read {}: {err}", path.display()))?;
    let mut lines: Vec<String> = content.split('\n').map(str::to_string).collect();

    let mut applied_ranges: Vec<(usize, usize)> = Vec::new();

    for fix in largest_range_first(fixes) {
        let overlaps_applied = applied_ranges
            .iter()
            .any(|&(start, end)| fix.start_line <= end && fix.end_line >= start);
        if overlaps_applied || fix.end_line > lines.len() || changes_nothing(&lines, &fix) {
            continue;
        }

        lines.splice(
            fix.start_line - 1..fix.end_line,
            fix.replacement.split('\n').map(str::to_string),
        );
        applied_ranges.push((fix.start_line, fix.end_line));
    }

    if !applied_ranges.is_empty() {
        originals.write(path, &content, &lines.join("\n"))?;
    }

    Ok(applied_ranges.len())
}

fn changes_nothing(lines: &[String], fix: &LineFix) -> bool {
    lines[fix.start_line - 1..fix.end_line].join("\n") == fix.replacement
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

    static NEXT_DIR: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    fn apply_to(content: &str, fixes: Vec<LineFix>) -> (usize, String) {
        let dir = std::env::temp_dir().join(format!(
            "klint-rs-fix-unit-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).expect("create dir");
        let path = dir.join("a.ts");
        fs::write(&path, content).expect("write");
        let applied = apply_file_fixes(&path, fixes, &mut Originals::default()).expect("apply");
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
    fn a_fix_that_changes_nothing_is_not_counted() {
        let (applied, result) = apply_to("a\nb\n", vec![fix(1, 1, "a"), fix(2, 2, "B")]);
        assert_eq!(applied, 1);
        assert_eq!(result, "a\nB\n");
    }

    #[test]
    fn stops_after_the_pass_limit_when_fixes_keep_coming() {
        let mut lint_calls = 0;
        let outcome = fix_until_stable(
            || {
                lint_calls += 1;
                Ok(())
            },
            |_| Ok(1),
            |_| true,
        )
        .expect("passes run");
        assert_eq!(outcome.applied, MAX_FIX_PASSES);
        assert!(!outcome.converged);
        assert_eq!(lint_calls, MAX_FIX_PASSES + 1);
    }

    #[test]
    fn converges_once_a_pass_applies_nothing() {
        let mut fixes_left = 3;
        let outcome = fix_until_stable(
            || Ok(()),
            |_| {
                let applied = usize::from(fixes_left > 0);
                fixes_left -= applied;
                Ok(applied)
            },
            |_| true,
        )
        .expect("passes run");
        assert_eq!(outcome.applied, 3);
        assert!(outcome.converged);
    }

    #[test]
    fn ignores_a_fix_past_the_end_of_the_file() {
        let (applied, result) = apply_to("a\n", vec![fix(9, 9, "x")]);
        assert_eq!(applied, 0);
        assert_eq!(result, "a\n");
    }

    fn scratch_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "klint-rs-fix-unit-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).expect("create dir");
        dir
    }

    #[test]
    fn restore_after_puts_every_written_file_back_and_counts_them() {
        let dir = scratch_dir();
        let mut originals = Originals::default();

        for name in ["a.ts", "b.ts"] {
            originals
                .write(&dir.join(name), "before\n", "after\n")
                .expect("write");
        }

        let message = originals.restore_after("klint: boom".to_string());

        assert_eq!(
            message,
            "klint: boom\nklint: restored 2 file(s) to their state before --fix"
        );

        for name in ["a.ts", "b.ts"] {
            assert_eq!(
                fs::read_to_string(dir.join(name)).expect("read"),
                "before\n"
            );
        }

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn restore_after_with_nothing_written_keeps_the_error_alone() {
        assert_eq!(
            Originals::default().restore_after("klint: boom".to_string()),
            "klint: boom"
        );
    }

    #[test]
    fn restore_after_names_a_file_it_could_not_restore() {
        let dir = scratch_dir();
        let path = dir.join("gone").join("a.ts");
        let mut originals = Originals::default();
        originals.0.insert(path.clone(), "before\n".to_string());

        let message = originals.restore_after("klint: boom".to_string());

        assert!(
            message.starts_with(&format!(
                "klint: boom\nklint: failed to restore {}: ",
                path.display()
            )),
            "{message}"
        );
        assert!(!message.contains("restored 1"), "{message}");
        let _ = fs::remove_dir_all(dir);
    }

    fn violation(file: &str, fix: Option<serde_json::Value>) -> Violation {
        Violation {
            file: file.to_string(),
            line: 1,
            rule: "no-string-match".to_string(),
            message: String::new(),
            severity: "error".to_string(),
            fix,
            span: None,
        }
    }

    fn line_fix_json(start_line: u64) -> Option<serde_json::Value> {
        Some(
            serde_json::json!({ "startLine": start_line, "endLine": start_line, "replacement": "x" }),
        )
    }

    #[test]
    fn has_fixes_needs_a_selected_violation_with_a_usable_fix() {
        let dir = scratch_dir();
        fs::write(dir.join("a.ts"), "a\n").expect("write");
        let root = dir.as_path();
        let everywhere = FixScope::default();
        let elsewhere = FixScope {
            safe_only: false,
            paths: Some(vec![dir.join("other")]),
        };

        assert!(has_fixes(
            &[violation("a.ts", line_fix_json(1))],
            root,
            &everywhere
        ));
        assert!(!has_fixes(&[violation("a.ts", None)], root, &everywhere));
        assert!(!has_fixes(
            &[violation("a.ts", line_fix_json(0))],
            root,
            &everywhere
        ));
        assert!(!has_fixes(
            &[violation("a.ts", line_fix_json(1))],
            root,
            &elsewhere
        ));
        let _ = fs::remove_dir_all(dir);
    }
}
