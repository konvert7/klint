mod comments;
mod imports;
mod layers;
mod patterns;
mod resolve;

use crate::files::relative_path;
use crate::output::Violation;
use crate::syntax::TreeCache;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use comments::run_arch_comment_rules;
use imports::run_arch_import_rules;
use layers::resolve_layer_files;
use patterns::{run_arch_forbidden_rules, run_arch_singleton_rules};

#[derive(Debug, Deserialize)]
pub(crate) struct ArchConfig {
    layers: Option<BTreeMap<String, Vec<String>>>,
    imports: Option<Vec<ArchImportRule>>,
    forbidden: Option<Vec<ArchForbiddenRule>>,
    singleton: Option<Vec<ArchSingletonRule>>,
    #[serde(rename = "maxLines")]
    max_lines: Option<Vec<ArchMaxLinesRule>>,
    #[serde(rename = "maxCommentDensity")]
    max_comment_density: Option<Vec<ArchMaxCommentDensityRule>>,
    #[serde(rename = "maxCommentBlock")]
    max_comment_block: Option<Vec<ArchMaxCommentBlockRule>>,
}

#[derive(Debug, Deserialize)]
struct ArchImportRule {
    from: StringOrVec,
    deny: Option<StringOrVec>,
    allow: Option<StringOrVec>,
    #[serde(rename = "deny-packages")]
    deny_packages: Option<StringOrVec>,
    #[serde(rename = "type-only")]
    type_only: Option<String>,
    message: Option<String>,
    severity: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ArchForbiddenRule {
    pattern: Option<String>,
    #[serde(rename = "jsx-element")]
    jsx_element: Option<StringOrVec>,
    #[serde(rename = "in")]
    in_scope: StringOrVec,
    message: String,
    severity: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ArchSingletonRule {
    pattern: Option<String>,
    #[serde(rename = "jsx-element")]
    jsx_element: Option<StringOrVec>,
    only: String,
    #[serde(rename = "in")]
    in_scope: Option<StringOrVec>,
    message: String,
    severity: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ArchMaxLinesRule {
    limit: usize,
    #[serde(rename = "in")]
    in_scope: StringOrVec,
    message: Option<String>,
    severity: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ArchMaxCommentDensityRule {
    limit: f64,
    #[serde(rename = "countDocComments", default)]
    count_doc_comments: bool,
    #[serde(rename = "in")]
    in_scope: StringOrVec,
    message: Option<String>,
    severity: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ArchMaxCommentBlockRule {
    limit: usize,
    #[serde(rename = "countDocComments", default)]
    count_doc_comments: bool,
    #[serde(rename = "in")]
    in_scope: StringOrVec,
    message: Option<String>,
    severity: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StringOrVec {
    One(String),
    Many(Vec<String>),
}

impl StringOrVec {
    fn items(&self) -> Vec<String> {
        match self {
            Self::One(item) => vec![item.clone()],
            Self::Many(items) => items.clone(),
        }
    }
}

/// Runs the four arch checks (imports, forbidden, singleton, max-lines) as
/// scoped threads — each already walks the full file list independently, so
/// this lets those walks happen concurrently. Results are joined back in
/// this fixed order regardless of thread completion order, so output
/// ordering is unaffected.
pub(crate) fn run_arch_rules(
    arch: &ArchConfig,
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    tree_cache: &TreeCache,
    root: &Path,
) -> Vec<Violation> {
    std::thread::scope(|scope| {
        let imports =
            scope.spawn(|| run_arch_import_rules(arch, files, file_contents, tree_cache, root));
        let forbidden =
            scope.spawn(|| run_arch_forbidden_rules(arch, files, file_contents, tree_cache, root));
        let singleton =
            scope.spawn(|| run_arch_singleton_rules(arch, files, file_contents, tree_cache, root));
        let max_lines = scope.spawn(|| run_arch_max_lines_rules(arch, files, file_contents, root));
        let comments =
            scope.spawn(|| run_arch_comment_rules(arch, files, file_contents, tree_cache, root));

        let mut violations = imports.join().expect("arch imports thread panicked");
        violations.extend(forbidden.join().expect("arch forbidden thread panicked"));
        violations.extend(singleton.join().expect("arch singleton thread panicked"));
        violations.extend(max_lines.join().expect("arch max-lines thread panicked"));
        violations.extend(comments.join().expect("arch comments thread panicked"));
        violations
    })
}

fn run_arch_max_lines_rules(
    arch: &ArchConfig,
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    root: &Path,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    let Some(rules) = &arch.max_lines else {
        return violations;
    };

    for rule in rules {
        let severity = rule.severity.as_deref().unwrap_or("error");
        let scoped_files = resolve_layer_files(&rule.in_scope, arch.layers.as_ref(), root, files);
        for file in scoped_files {
            let Some(content) = file_contents.get(&file) else {
                continue;
            };
            if content.lines().count() > rule.limit {
                let message = rule
                    .message
                    .clone()
                    .unwrap_or_else(|| format!("File exceeds the maximum of {} lines", rule.limit));
                violations.push(Violation {
                    file: relative_path(root, &file),
                    line: rule.limit + 1,
                    rule: "arch/max-lines".to_string(),
                    message,
                    severity: severity.to_string(),
                    fix: None,
                });
            }
        }
    }
    violations
}
