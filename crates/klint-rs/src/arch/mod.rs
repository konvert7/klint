mod comments;
mod imports;
mod layers;
mod patterns;
mod resolve;

use crate::files::relative_path;
use crate::output::Violation;
use crate::syntax::CommentRecord;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tree_sitter::Node;

use comments::{CommentLines, CommentPass, plan_block_passes, plan_density_passes};
use imports::{ImportPass, ResolveContext, plan_import_passes};
use layers::resolve_layer_mask;
use patterns::{PatternPass, plan_forbidden_passes, plan_singleton_passes};

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
    ignore: Option<StringOrVec>,
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
    ignore: Option<StringOrVec>,
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

struct MaxLinesPass {
    limit: usize,
    scope: Vec<bool>,
    message: Option<String>,
    severity: String,
}

/// Every arch rule resolved into a per-file pass, with project-wide indexes
/// and layer scopes built once instead of once per rule. Pass order here is
/// the order violations are emitted in.
pub(crate) struct ArchPlan {
    imports: Vec<ImportPass>,
    forbidden: Vec<PatternPass>,
    singleton: Vec<PatternPass>,
    max_lines: Vec<MaxLinesPass>,
    density: Vec<CommentPass>,
    block: Vec<CommentPass>,
    resolve: ResolveContext,
}

impl ArchPlan {
    pub(crate) fn build(arch: &ArchConfig, files: &[PathBuf], root: &Path) -> Self {
        Self {
            imports: plan_import_passes(arch, files, root),
            forbidden: plan_forbidden_passes(arch, files, root),
            singleton: plan_singleton_passes(arch, files, root),
            max_lines: plan_max_lines_passes(arch, files, root),
            density: plan_density_passes(arch, files, root),
            block: plan_block_passes(arch, files, root),
            resolve: ResolveContext::build(root, files),
        }
    }

    pub(crate) fn pass_count(&self) -> usize {
        self.imports.len()
            + self.forbidden.len()
            + self.singleton.len()
            + self.max_lines.len()
            + self.density.len()
            + self.block.len()
    }

    /// Whether any arch pass covering this file needs its AST.
    pub(crate) fn needs_tree(&self, file_index: usize, file: &Path) -> bool {
        self.imports
            .iter()
            .any(|pass| pass.covers(file_index, file))
            || self
                .forbidden
                .iter()
                .chain(&self.singleton)
                .any(|pass| pass.needs_tree() && pass.covers(file_index, file))
            || self.needs_comments(file_index)
    }

    fn needs_comments(&self, file_index: usize) -> bool {
        self.density
            .iter()
            .chain(&self.block)
            .any(|pass| pass.covers(file_index))
    }

    /// Runs every arch pass over one file, appending one bucket per pass in
    /// emission order.
    pub(crate) fn scan_file(&self, scan: ArchFileScan<'_>, out: &mut Vec<Vec<Violation>>) {
        let ArchFileScan {
            file_index,
            file,
            root,
            tree_root,
            source,
            content,
        } = scan;

        for pass in &self.imports {
            out.push(match tree_root {
                Some(node) if pass.covers(file_index, file) => {
                    pass.scan(file, root, node, source, &self.resolve)
                }
                _ => Vec::new(),
            });
        }

        for pass in self.forbidden.iter().chain(&self.singleton) {
            out.push(if pass.covers(file_index, file) {
                pass.scan(file, root, tree_root, source, content)
            } else {
                Vec::new()
            });
        }

        for pass in &self.max_lines {
            out.push(pass.scan(file_index, file, root, content));
        }

        let comment_lines = (self.needs_comments(file_index) && tree_root.is_some()).then(|| {
            let records: Vec<CommentRecord> = tree_root
                .map(|node| crate::syntax::scan_comments_from_tree(file, node, source))
                .unwrap_or_default();
            CommentLines::from_records(&records)
        });

        for pass in self.density.iter().chain(&self.block) {
            out.push(match (&comment_lines, pass.covers(file_index)) {
                (Some(lines), true) => pass.scan(file, root, content, lines),
                _ => Vec::new(),
            });
        }
    }
}

/// One file's inputs for an arch pass sweep.
pub(crate) struct ArchFileScan<'a> {
    pub file_index: usize,
    pub file: &'a Path,
    pub root: &'a Path,
    pub tree_root: Option<Node<'a>>,
    pub source: &'a [u8],
    pub content: &'a str,
}

fn plan_max_lines_passes(arch: &ArchConfig, files: &[PathBuf], root: &Path) -> Vec<MaxLinesPass> {
    let Some(rules) = &arch.max_lines else {
        return Vec::new();
    };
    rules
        .iter()
        .map(|rule| MaxLinesPass {
            limit: rule.limit,
            scope: resolve_layer_mask(&rule.in_scope, arch.layers.as_ref(), root, files),
            message: rule.message.clone(),
            severity: rule.severity.as_deref().unwrap_or("error").to_string(),
        })
        .collect()
}

impl MaxLinesPass {
    fn scan(&self, file_index: usize, file: &Path, root: &Path, content: &str) -> Vec<Violation> {
        if !self.scope.get(file_index).copied().unwrap_or(false) {
            return Vec::new();
        }
        if content.lines().count() <= self.limit {
            return Vec::new();
        }
        vec![Violation {
            file: relative_path(root, file),
            line: self.limit + 1,
            rule: "arch/max-lines".to_string(),
            message: self
                .message
                .clone()
                .unwrap_or_else(|| format!("File exceeds the maximum of {} lines", self.limit)),
            severity: self.severity.clone(),
            fix: None,
        }]
    }
}
