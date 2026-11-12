mod comments;
mod imports;
mod layers;
mod patterns;
mod resolve;
mod schema_shapes;

use crate::files::relative_path;
use crate::output::Violation;
use crate::syntax::CommentRecord;
use schema_shapes::{ArchSeverity, COMMENT_IGNORE, COUNT_DOC_COMMENTS, TypeOnly};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tree_sitter::Node;

use comments::{CommentLines, CommentPass, plan_block_passes, plan_density_passes};
use imports::{ImportPass, plan_import_passes};
use layers::resolve_layer_mask;
use patterns::{PatternPass, plan_forbidden_passes, plan_singleton_passes};
use resolve::ResolveContext;

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(
    deny_unknown_fields,
    description = "Architecture-as-Code constraints — layers, import boundaries, forbidden patterns, singleton locations, comment budgets. Enforced by the arch engine alongside the rule set."
)]
pub(crate) struct ArchConfig {
    #[schemars(
        description = "Named architectural zones. Keys are arbitrary layer names; values are glob patterns matching the files in that layer. Other arch rules reference these names instead of repeating globs.",
        extend("examples" = ["layers:\n  core:   [\"src/hooks/lib/**\", \"src/tools/**\"]\n  skills: [\"assets/skills/**\"]\n  dao:    [\"src/dao/**\"]"])
    )]
    layers: Option<BTreeMap<String, Vec<String>>>,
    #[schemars(
        description = "Import-boundary rules. The dependency graph your README claims you have."
    )]
    imports: Option<Vec<ArchImportRule>>,
    #[schemars(description = "Forbidden-pattern rules. Block literal strings scoped to a layer.")]
    forbidden: Option<Vec<ArchForbiddenRule>>,
    #[schemars(
        description = "Singleton-location rules. Pin a pattern to one file; every other touch is a violation."
    )]
    singleton: Option<Vec<ArchSingletonRule>>,
    #[serde(rename = "maxLines")]
    #[schemars(
        description = "Per-file line-count limits. Cap how long files in a layer may grow, with different ceilings per scope."
    )]
    max_lines: Option<Vec<ArchMaxLinesRule>>,
    #[serde(rename = "maxCommentDensity")]
    #[schemars(
        description = "Per-file comment-density ceilings. Cap the share of a file that may be explanatory comments; doc-comments are exempt by default."
    )]
    max_comment_density: Option<Vec<ArchMaxCommentDensityRule>>,
    #[serde(rename = "maxCommentBlock")]
    #[schemars(
        description = "Per-block comment-height caps. Limit how many comment lines may stack together; doc-comments are exempt by default."
    )]
    max_comment_block: Option<Vec<ArchMaxCommentBlockRule>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(
    deny_unknown_fields,
    description = "An import-boundary rule. Restricts which layers a source can or cannot import from.",
    extend("examples" = [
        "arch:\n  imports:\n    - from: skills\n      deny: core\n      message: \"Skills must be self-contained and portable\"\n      severity: warn",
        "arch:\n  imports:\n    - from: [\"src/dao/**\"]\n      allow: [\"src/dao/**\", \"src/prisma/**\", \"src/types/**\"]"
    ])
)]
struct ArchImportRule {
    #[schemars(
        description = "Source — a layer name or glob(s) for the files this rule applies to. Imports originating in `from` are checked against `deny`/`allow`."
    )]
    from: StringOrVec,
    #[schemars(
        description = "Layer name(s) or glob(s) the source is not allowed to import from. Mutually exclusive in spirit with `allow` (use one or the other)."
    )]
    deny: Option<StringOrVec>,
    #[schemars(
        description = "Allowlist of layer name(s) / glob(s) the source may import from. Anything not matching `allow` is denied."
    )]
    allow: Option<StringOrVec>,
    #[serde(rename = "deny-packages")]
    #[schemars(
        description = "Package specifier(s) the source is not allowed to import — npm packages, `node:` builtins, Python pip packages and stdlib modules, Swift system frameworks, external Rust crates, and C# namespaces or NuGet packages, which `deny`/`allow` cannot reach because they never resolve to a project file. Matching is per segment: `/` in TypeScript/JavaScript, so `next` also covers `next/headers` while `nextra` is unaffected; `.` in Python, Swift, and C#, so `os` also covers `os.path` while `oscrypto` is unaffected; and `::` in Rust, so `tokio` also covers `tokio::sync::Mutex` while `tokio_util` is unaffected. Python, Swift, Rust, and C# sources are checked by the Rust engine."
    )]
    deny_packages: Option<StringOrVec>,
    #[serde(rename = "type-only")]
    #[schemars(
        with = "Option<TypeOnly>",
        description = "When set to \"allow\", `import type` statements that would otherwise be denied are permitted. In Python, imports inside an `if TYPE_CHECKING:` block are exempted the same way. Useful for type-only references across layers."
    )]
    type_only: Option<String>,
    #[schemars(
        description = "Custom error message reported when the rule is violated. Shown in the agent's event stream alongside file:line."
    )]
    message: Option<String>,
    #[schemars(with = "Option<ArchSeverity>")]
    severity: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(
    with = "schema_shapes::ForbiddenRule",
    description = "A forbidden-pattern rule. Blocks either a literal substring (`pattern:`) or a JSX element (`jsx-element:`) inside specific layers."
)]
struct ArchForbiddenRule {
    pattern: Option<String>,
    #[serde(rename = "jsx-element")]
    jsx_element: Option<StringOrVec>,
    #[serde(rename = "in")]
    in_scope: StringOrVec,
    message: String,
    severity: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(
    with = "schema_shapes::SingletonRule",
    description = "A singleton-location rule. Pins a pattern or JSX element to exactly one file — the only honest way to enforce a module of record or a design-system primitive."
)]
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

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(
    deny_unknown_fields,
    description = "A per-file line-count limit. Caps how long any file in scope may grow — the honest way to keep modules small without a formatter dependency.",
    extend("examples" = [
        "arch:\n  maxLines:\n    - limit: 300\n      in: src/**\n      message: \"Split this module\"",
        "arch:\n  maxLines:\n    - limit: 300\n      in: src/**\n    - limit: 600\n      in: tests/**"
    ])
)]
struct ArchMaxLinesRule {
    #[schemars(
        range(min = 1),
        description = "Maximum number of physical lines a file may contain. Counts every line including blanks and comments. A file with more lines than this is a violation."
    )]
    limit: usize,
    #[serde(rename = "in")]
    #[schemars(
        description = "Layer name(s) or glob(s) the limit applies to. Files outside the scope are not checked."
    )]
    in_scope: StringOrVec,
    #[schemars(
        description = "Optional error message. Defaults to a message naming the limit (e.g. 'File exceeds the maximum of 300 lines')."
    )]
    message: Option<String>,
    #[schemars(with = "Option<ArchSeverity>")]
    severity: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(
    deny_unknown_fields,
    description = "A per-file comment-density ceiling. Caps how much of a file may be explanatory comments — the honest way to push intent into code and names rather than prose (and to keep token cost down), while leaving doc-comments free by default.",
    extend("examples" = [
        "arch:\n  maxCommentDensity:\n    - limit: 5\n      in: src/**\n      message: \"Encode intent in names, not comments\"",
        "arch:\n  maxCommentDensity:\n    - limit: 10\n      countDocComments: true\n      in: src/**"
    ])
)]
struct ArchMaxCommentDensityRule {
    #[schemars(
        extend("exclusiveMinimum" = 0, "maximum" = 100),
        description = "Maximum percentage of a file's physical lines that may be comment lines. Denominator is total physical lines (code + comments + blanks), the same count `maxLines` uses. A file above this ratio is a violation."
    )]
    limit: f64,
    #[serde(rename = "countDocComments", default)]
    #[schemars(with = "Option<bool>", description = COUNT_DOC_COMMENTS)]
    count_doc_comments: bool,
    #[schemars(description = COMMENT_IGNORE)]
    ignore: Option<StringOrVec>,
    #[serde(rename = "in")]
    #[schemars(
        description = "Layer name(s) or glob(s) the limit applies to. Files outside the scope are not checked."
    )]
    in_scope: StringOrVec,
    #[schemars(
        description = "Optional error message. Defaults to a message naming the measured density and the limit."
    )]
    message: Option<String>,
    #[schemars(with = "Option<ArchSeverity>")]
    severity: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(
    deny_unknown_fields,
    description = "A per-block comment-height cap. Limits how many comment lines may stack together, so comments stay terse and a long explanation becomes a well-named function instead. Doc-comments are exempt by default.",
    extend("examples" = [
        "arch:\n  maxCommentBlock:\n    - limit: 2\n      in: src/**\n      message: \"Extract a named function instead of a comment block\""
    ])
)]
struct ArchMaxCommentBlockRule {
    #[schemars(
        range(min = 1),
        description = "Maximum number of consecutive comment lines allowed in a single block. A run of more comment lines than this — e.g. a tall comment stacked above a statement — is a violation, reported at the first offending line."
    )]
    limit: usize,
    #[serde(rename = "countDocComments", default)]
    #[schemars(with = "Option<bool>", description = COUNT_DOC_COMMENTS)]
    count_doc_comments: bool,
    #[schemars(description = COMMENT_IGNORE)]
    ignore: Option<StringOrVec>,
    #[serde(rename = "in")]
    #[schemars(
        description = "Layer name(s) or glob(s) the limit applies to. Files outside the scope are not checked."
    )]
    in_scope: StringOrVec,
    #[schemars(
        description = "Optional error message. Defaults to a message naming the consecutive-line limit."
    )]
    message: Option<String>,
    #[schemars(with = "Option<ArchSeverity>")]
    severity: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(
    description = "Either a single layer name / glob pattern or an array of them. Layer names defined in `arch.layers` are resolved to their glob lists."
)]
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
