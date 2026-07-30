use super::layers::resolve_layer_mask;
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
    message: Option<String>,
    severity: String,
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
        let hit = match self.limit {
            CommentLimit::Density(limit) => density_overrun(content, lines, limit),
            CommentLimit::Block(limit) => {
                first_comment_block_overrun(lines, limit).map(|line| (line, block_message(limit)))
            }
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

    fn rule_name(&self) -> &'static str {
        match self.limit {
            CommentLimit::Density(_) => "arch/max-comment-density",
            CommentLimit::Block(_) => "arch/max-comment-block",
        }
    }
}

fn density_overrun(content: &str, lines: &[usize], limit: f64) -> Option<(usize, String)> {
    let total = content.lines().count();
    if total == 0 {
        return None;
    }
    let density = (lines.len() as f64 / total as f64) * 100.0;
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

/// First line at which a run of consecutive comment lines exceeds `limit`.
/// Mirrors the TS engine's `firstCommentBlockOverrun`.
fn first_comment_block_overrun(sorted_lines: &[usize], limit: usize) -> Option<usize> {
    let (&first, rest) = sorted_lines.split_first()?;
    let mut run_start = first;
    let mut prev = first;
    for &line in rest {
        if line == prev + 1 {
            prev = line;
            continue;
        }
        if prev - run_start + 1 > limit {
            return Some(run_start + limit);
        }
        run_start = line;
        prev = line;
    }
    if prev - run_start + 1 > limit {
        return Some(run_start + limit);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::first_comment_block_overrun;

    #[test]
    fn reports_first_line_past_the_limit_in_an_over_tall_block() {
        assert_eq!(first_comment_block_overrun(&[5, 6, 7, 8], 2), Some(7));
    }

    #[test]
    fn accepts_a_block_exactly_at_the_limit() {
        assert_eq!(first_comment_block_overrun(&[5, 6], 2), None);
    }

    #[test]
    fn a_gap_breaks_the_run() {
        assert_eq!(first_comment_block_overrun(&[1, 2, 4, 5], 2), None);
    }

    #[test]
    fn flags_an_over_tall_run_that_ends_the_file() {
        assert_eq!(first_comment_block_overrun(&[1, 2, 5, 6, 7], 2), Some(7));
    }

    #[test]
    fn empty_input_never_flags() {
        assert_eq!(first_comment_block_overrun(&[], 2), None);
    }
}
