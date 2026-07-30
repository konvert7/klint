use super::layers::resolve_layer_mask;
use super::patterns::LineMatcher;
use super::*;
use crate::files::relative_path;
use crate::output::Violation;
use crate::syntax::CommentRecord;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Comment lines for one file, derived once per file and shared by every
/// comment pass rather than re-walking the tree per rule.
pub(super) struct CommentLines {
    with_docs: Vec<usize>,
    without_docs: Vec<usize>,
}

impl CommentLines {
    pub(super) fn from_records(records: &[CommentRecord]) -> Self {
        Self {
            with_docs: line_set(records, true),
            without_docs: line_set(records, false),
        }
    }

    fn lines(&self, count_doc_comments: bool) -> &[usize] {
        if count_doc_comments {
            &self.with_docs
        } else {
            &self.without_docs
        }
    }
}

fn line_set(records: &[CommentRecord], count_doc_comments: bool) -> Vec<usize> {
    let mut lines = BTreeSet::new();
    for comment in records {
        if !count_doc_comments && comment.is_doc {
            continue;
        }
        for line in comment.start_line..=comment.end_line {
            lines.insert(line);
        }
    }
    lines.into_iter().collect()
}

enum CommentLimit {
    Density(f64),
    Block(usize),
}

pub(super) struct CommentPass {
    limit: CommentLimit,
    scope: Vec<bool>,
    count_doc_comments: bool,
    ignore: Vec<LineMatcher>,
    message: Option<String>,
    severity: String,
}

fn build_ignore(ignore: Option<&StringOrVec>) -> Vec<LineMatcher> {
    ignore
        .map(StringOrVec::items)
        .unwrap_or_default()
        .iter()
        .map(|pattern| LineMatcher::build(pattern))
        .collect()
}

pub(super) fn plan_density_passes(
    arch: &ArchConfig,
    files: &[PathBuf],
    root: &Path,
) -> Vec<CommentPass> {
    let Some(rules) = &arch.max_comment_density else {
        return Vec::new();
    };
    rules
        .iter()
        .map(|rule| CommentPass {
            limit: CommentLimit::Density(rule.limit),
            scope: resolve_layer_mask(&rule.in_scope, arch.layers.as_ref(), root, files),
            count_doc_comments: rule.count_doc_comments,
            ignore: build_ignore(rule.ignore.as_ref()),
            message: rule.message.clone(),
            severity: rule.severity.as_deref().unwrap_or("error").to_string(),
        })
        .collect()
}

pub(super) fn plan_block_passes(
    arch: &ArchConfig,
    files: &[PathBuf],
    root: &Path,
) -> Vec<CommentPass> {
    let Some(rules) = &arch.max_comment_block else {
        return Vec::new();
    };
    rules
        .iter()
        .map(|rule| CommentPass {
            limit: CommentLimit::Block(rule.limit),
            scope: resolve_layer_mask(&rule.in_scope, arch.layers.as_ref(), root, files),
            count_doc_comments: rule.count_doc_comments,
            ignore: build_ignore(rule.ignore.as_ref()),
            message: rule.message.clone(),
            severity: rule.severity.as_deref().unwrap_or("error").to_string(),
        })
        .collect()
}

impl CommentPass {
    pub(super) fn covers(&self, file_index: usize) -> bool {
        self.scope.get(file_index).copied().unwrap_or(false)
    }

    pub(super) fn scan(
        &self,
        file: &Path,
        root: &Path,
        content: &str,
        comments: &CommentLines,
    ) -> Vec<Violation> {
        let lines = comments.lines(self.count_doc_comments);
        let ignored = self.ignored_lines(content, lines);
        let hit = match self.limit {
            CommentLimit::Density(limit) => density_overrun(content, lines, &ignored, limit),
            CommentLimit::Block(limit) => first_comment_block_overrun(lines, limit, &ignored)
                .map(|line| (line, block_message(limit))),
        };

        hit.map(|(line, fallback)| Violation {
            file: relative_path(root, file),
            line,
            rule: self.rule_name().to_string(),
            message: self.message.clone().unwrap_or(fallback),
            severity: self.severity.clone(),
            fix: None,
        })
        .into_iter()
        .collect()
    }

    fn ignored_lines(&self, content: &str, comment_lines: &[usize]) -> BTreeSet<usize> {
        if self.ignore.is_empty() {
            return BTreeSet::new();
        }
        let source: Vec<&str> = content.lines().collect();
        comment_lines
            .iter()
            .copied()
            .filter(|line| {
                let text = source.get(line - 1).copied().unwrap_or("");
                self.ignore.iter().any(|matcher| matcher.is_match(text))
            })
            .collect()
    }

    fn rule_name(&self) -> &'static str {
        match self.limit {
            CommentLimit::Density(_) => "arch/max-comment-density",
            CommentLimit::Block(_) => "arch/max-comment-block",
        }
    }
}

fn density_overrun(
    content: &str,
    lines: &[usize],
    ignored: &BTreeSet<usize>,
    limit: f64,
) -> Option<(usize, String)> {
    let total = content.lines().count();
    if total == 0 {
        return None;
    }
    let counted = lines.len() - ignored.len();
    let density = (counted as f64 / total as f64) * 100.0;
    (density > limit).then(|| {
        (
            1,
            format!("Comment density {density:.1}% exceeds the maximum of {limit}%"),
        )
    })
}

fn block_message(limit: usize) -> String {
    format!("Comment block exceeds the maximum of {limit} consecutive lines")
}

/// First line at which a run of consecutive comment lines exceeds `limit`. An
/// ignored line neither counts toward the height nor breaks the run.
/// Mirrors the TS engine's `firstCommentBlockOverrun`.
fn first_comment_block_overrun(
    sorted_lines: &[usize],
    limit: usize,
    ignored: &BTreeSet<usize>,
) -> Option<usize> {
    let mut counted = 0;
    let mut prev: Option<usize> = None;
    for &line in sorted_lines {
        if prev.is_some_and(|prev| line != prev + 1) {
            counted = 0;
        }
        prev = Some(line);
        if ignored.contains(&line) {
            continue;
        }
        counted += 1;
        if counted > limit {
            return Some(line);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::first_comment_block_overrun;
    use std::collections::BTreeSet;

    fn overrun(lines: &[usize], limit: usize) -> Option<usize> {
        first_comment_block_overrun(lines, limit, &BTreeSet::new())
    }

    fn overrun_ignoring(lines: &[usize], limit: usize, ignored: &[usize]) -> Option<usize> {
        first_comment_block_overrun(lines, limit, &ignored.iter().copied().collect())
    }

    #[test]
    fn reports_first_line_past_the_limit_in_an_over_tall_block() {
        assert_eq!(overrun(&[5, 6, 7, 8], 2), Some(7));
    }

    #[test]
    fn accepts_a_block_exactly_at_the_limit() {
        assert_eq!(overrun(&[5, 6], 2), None);
    }

    #[test]
    fn a_gap_breaks_the_run() {
        assert_eq!(overrun(&[1, 2, 4, 5], 2), None);
    }

    #[test]
    fn flags_an_over_tall_run_that_ends_the_file() {
        assert_eq!(overrun(&[1, 2, 5, 6, 7], 2), Some(7));
    }

    #[test]
    fn empty_input_never_flags() {
        assert_eq!(overrun(&[], 2), None);
    }

    #[test]
    fn an_ignored_line_does_not_count_toward_the_height() {
        assert_eq!(overrun_ignoring(&[1, 2, 3], 2, &[2]), None);
    }

    #[test]
    fn an_ignored_line_does_not_break_the_run() {
        assert_eq!(overrun_ignoring(&[1, 2, 3, 4], 2, &[2]), Some(4));
    }

    #[test]
    fn a_fully_ignored_block_never_flags() {
        assert_eq!(overrun_ignoring(&[1, 2, 3, 4], 1, &[1, 2, 3, 4]), None);
    }

    #[test]
    fn a_real_gap_still_breaks_a_run_containing_ignored_lines() {
        assert_eq!(overrun_ignoring(&[1, 2, 3, 7, 8, 9], 2, &[2, 8]), None);
    }
}
