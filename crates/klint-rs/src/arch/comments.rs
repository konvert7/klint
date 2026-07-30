use super::*;
use crate::files::relative_path;
use crate::output::Violation;
use crate::syntax::{TreeCache, scan_comments_from_tree};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub(super) fn run_arch_comment_rules(
    arch: &ArchConfig,
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    tree_cache: &TreeCache,
    root: &Path,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    run_arch_max_comment_density_rules(
        arch,
        files,
        file_contents,
        tree_cache,
        root,
        &mut violations,
    );
    run_arch_max_comment_block_rules(
        arch,
        files,
        file_contents,
        tree_cache,
        root,
        &mut violations,
    );
    violations
}

/// Sorted, de-duplicated 1-based line numbers touched by comments in `file`,
/// with doc-comments dropped unless `count_doc_comments` is set. `None` when
/// the file cannot be parsed.
fn comment_line_set(
    file: &Path,
    content: &str,
    tree_cache: &TreeCache,
    count_doc_comments: bool,
) -> Option<Vec<usize>> {
    let tree = tree_cache.get_or_parse(file, content)?;
    let comments = scan_comments_from_tree(file, tree.root_node(), content.as_bytes());
    let mut lines = std::collections::BTreeSet::new();
    for comment in comments {
        if !count_doc_comments && comment.is_doc {
            continue;
        }
        for line in comment.start_line..=comment.end_line {
            lines.insert(line);
        }
    }
    Some(lines.into_iter().collect())
}

fn run_arch_max_comment_density_rules(
    arch: &ArchConfig,
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    tree_cache: &TreeCache,
    root: &Path,
    violations: &mut Vec<Violation>,
) {
    let Some(rules) = &arch.max_comment_density else {
        return;
    };
    for rule in rules {
        let severity = rule.severity.as_deref().unwrap_or("error");
        let scoped_files = resolve_layer_files(&rule.in_scope, arch.layers.as_ref(), root, files);
        for file in scoped_files {
            let Some(content) = file_contents.get(&file) else {
                continue;
            };
            let total = content.lines().count();
            if total == 0 {
                continue;
            }
            let Some(comment_lines) =
                comment_line_set(&file, content, tree_cache, rule.count_doc_comments)
            else {
                continue;
            };
            let density = (comment_lines.len() as f64 / total as f64) * 100.0;
            if density > rule.limit {
                let message = rule.message.clone().unwrap_or_else(|| {
                    format!(
                        "Comment density {:.1}% exceeds the maximum of {}%",
                        density, rule.limit
                    )
                });
                violations.push(Violation {
                    file: relative_path(root, &file),
                    line: 1,
                    rule: "arch/max-comment-density".to_string(),
                    message,
                    severity: severity.to_string(),
                    fix: None,
                });
            }
        }
    }
}

fn run_arch_max_comment_block_rules(
    arch: &ArchConfig,
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    tree_cache: &TreeCache,
    root: &Path,
    violations: &mut Vec<Violation>,
) {
    let Some(rules) = &arch.max_comment_block else {
        return;
    };
    for rule in rules {
        let severity = rule.severity.as_deref().unwrap_or("error");
        let scoped_files = resolve_layer_files(&rule.in_scope, arch.layers.as_ref(), root, files);
        for file in scoped_files {
            let Some(content) = file_contents.get(&file) else {
                continue;
            };
            let Some(comment_lines) =
                comment_line_set(&file, content, tree_cache, rule.count_doc_comments)
            else {
                continue;
            };
            if let Some(line) = first_comment_block_overrun(&comment_lines, rule.limit) {
                let message = rule.message.clone().unwrap_or_else(|| {
                    format!(
                        "Comment block exceeds the maximum of {} consecutive lines",
                        rule.limit
                    )
                });
                violations.push(Violation {
                    file: relative_path(root, &file),
                    line,
                    rule: "arch/max-comment-block".to_string(),
                    message,
                    severity: severity.to_string(),
                    fix: None,
                });
            }
        }
    }
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
