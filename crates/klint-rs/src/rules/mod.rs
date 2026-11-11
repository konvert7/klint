mod blank_lines;
mod builtin;
mod docs;
mod sonar;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::config::RuleConfig;
use crate::engine::{NodeIndex, Wants};
use crate::files::{is_javascript_like_source, match_pattern, relative_path, supports_import_scan};
use crate::output::Violation;
use crate::syntax::SourceLanguage;
pub(crate) use docs::RULE_DOCS;

#[derive(Clone, Copy)]
enum FileScope {
    JavaScriptLike,
    AnySource,
}

pub(crate) enum RuleOptions {
    None,
    BlankLines(blank_lines::BlankLineOptions),
}

pub(crate) struct ParsedOptions {
    pub options: RuleOptions,
    pub problems: Vec<String>,
}

type RuleOptionsParser = fn(&'static str, &RuleConfig) -> ParsedOptions;

#[derive(Clone, Copy)]
struct RuleSpec {
    name: &'static str,
    check: RuleCheck,
    wants: RuleWants,
    scope: FileScope,
    options: RuleOptionsParser,
}

impl RuleSpec {
    const fn javascript(name: &'static str, check: RuleCheck, wants: RuleWants) -> Self {
        Self {
            name,
            check,
            wants,
            scope: FileScope::JavaScriptLike,
            options: no_options,
        }
    }
}

fn no_options(_rule: &'static str, _config: &RuleConfig) -> ParsedOptions {
    ParsedOptions {
        options: RuleOptions::None,
        problems: Vec::new(),
    }
}

pub(crate) fn rule_option_problems(name: &str, config: &RuleConfig) -> Vec<String> {
    CHECKS
        .iter()
        .find(|spec| spec.name == name)
        .map(|spec| (spec.options)(spec.name, config).problems)
        .unwrap_or_default()
}

/// One file's indexed nodes, handed to every rule that applies to it. Rules
/// read the buckets they care about instead of each walking the tree.
#[derive(Clone, Copy)]
pub(crate) struct RuleScan<'a> {
    pub index: &'a NodeIndex<'a>,
    pub source: &'a [u8],
    pub content: &'a str,
    pub language: SourceLanguage,
    pub options: &'a RuleOptions,
}

/// What a rule has to say about one scanned record. The file, rule id, and
/// severity are the same for every record a check produces, so [`RulePass::scan`]
/// fills those in.
pub(super) struct Report {
    pub line: usize,
    pub message: String,
    pub fix: Option<serde_json::Value>,
}

type RuleCheck = fn(RuleScan<'_>) -> Vec<Report>;

/// Which node buckets a rule reads, folded into the walk's [`Wants`].
type RuleWants = fn(&mut Wants);

/// Every rule the Rust engine supports, in the order their violations are
/// emitted.
const CHECKS: &[RuleSpec] = &[
    RuleSpec::javascript("no-string-match", builtin::run_no_string_match, |wants| {
        wants.calls = true;
    }),
    RuleSpec::javascript(
        "no-nested-template-literals",
        builtin::run_no_nested_template_literals,
        |wants| wants.templates = true,
    ),
    RuleSpec::javascript(
        "no-consecutive-array-push",
        builtin::run_no_consecutive_array_push,
        |wants| wants.containers = true,
    ),
    RuleSpec::javascript(
        "no-unguarded-json-parse",
        builtin::run_no_unguarded_json_parse,
        |wants| {
            wants.calls = true;
            wants.inside_try = true;
        },
    ),
    RuleSpec::javascript(
        "no-async-predicate",
        builtin::run_no_async_predicate,
        |wants| wants.calls = true,
    ),
    RuleSpec::javascript("no-sync-in-async", builtin::run_no_sync_in_async, |wants| {
        wants.calls = true;
        wants.in_async = true;
    }),
    RuleSpec::javascript(
        "sonar/no-single-char-class",
        sonar::run_sonar_no_single_char_class,
        |wants| wants.regexes = true,
    ),
    RuleSpec::javascript("sonar/prefer-at", sonar::run_sonar_prefer_at, |wants| {
        wants.subscripts = true
    }),
    RuleSpec::javascript(
        "sonar/prefer-string-replaceall",
        sonar::run_sonar_prefer_string_replaceall,
        |wants| wants.calls = true,
    ),
    RuleSpec::javascript(
        "sonar/prefer-string-raw",
        sonar::run_sonar_prefer_string_raw,
        |wants| wants.strings = true,
    ),
    RuleSpec::javascript(
        "sonar/prefer-string-raw-regexp",
        sonar::run_sonar_prefer_string_raw_regexp,
        |wants| wants.new_expressions = true,
    ),
    RuleSpec::javascript(
        "sonar/prefer-nullish-coalescing-assign",
        sonar::run_sonar_prefer_nullish_coalescing_assign,
        |wants| wants.if_statements = true,
    ),
    RuleSpec {
        name: "blank-line-before-control-flow",
        check: blank_lines::run_blank_line_before_control_flow,
        wants: |wants| wants.statement_blocks = true,
        scope: FileScope::AnySource,
        options: blank_lines::blank_line_options,
    },
    RuleSpec {
        name: "blank-line-after-block",
        check: blank_lines::run_blank_line_after_block,
        wants: |wants| wants.statement_blocks = true,
        scope: FileScope::AnySource,
        options: blank_lines::blank_line_options,
    },
];

/// A configured rule bound to the files it applies to. `applies` is indexed by
/// position in the engine's global file list, so per-file applicability is a
/// lookup rather than a glob match.
pub(crate) struct RulePass {
    rule: &'static str,
    check: RuleCheck,
    wants: RuleWants,
    severity: String,
    applies: Vec<bool>,
    options: RuleOptions,
}

impl RulePass {
    pub(crate) fn covers(&self, file_index: usize) -> bool {
        self.applies.get(file_index).copied().unwrap_or(false)
    }

    pub(crate) fn options(&self) -> &RuleOptions {
        &self.options
    }

    pub(crate) fn scan(&self, file: &Path, root: &Path, scan: RuleScan<'_>) -> Vec<Violation> {
        (self.check)(scan)
            .into_iter()
            .map(|report| Violation {
                file: relative_path(root, file),
                line: report.line,
                rule: self.rule.to_string(),
                message: report.message,
                severity: self.severity.clone(),
                fix: report.fix,
            })
            .collect()
    }
}

/// The configured rules in [`CHECKS`] order, each carrying the files it covers.
pub(crate) fn is_supported_rule(name: &str) -> bool {
    CHECKS.iter().any(|spec| spec.name == name)
}

pub(crate) fn plan_rule_passes(
    rules: &BTreeMap<String, RuleConfig>,
    files: &[PathBuf],
    root: &Path,
) -> Vec<RulePass> {
    CHECKS
        .iter()
        .filter_map(|spec| {
            let config = rules.get(spec.name)?;
            let severity = config.severity();
            if severity == "off" {
                return None;
            }
            Some(RulePass {
                rule: spec.name,
                check: spec.check,
                wants: spec.wants,
                severity: severity.to_string(),
                applies: files
                    .iter()
                    .map(|file| rule_applies_to_file(spec.scope, config, root, file))
                    .collect(),
                options: (spec.options)(spec.name, config).options,
            })
        })
        .collect()
}

/// The union of every configured rule's bucket needs, so the walk fills only
/// what something will read.
pub(crate) fn rule_wants(passes: &[RulePass]) -> Wants {
    let mut wants = Wants::default();
    for pass in passes {
        (pass.wants)(&mut wants);
    }
    wants
}

fn rule_applies_to_file(scope: FileScope, config: &RuleConfig, root: &Path, file: &Path) -> bool {
    if !scope_covers_file(scope, file) {
        return false;
    }

    let Some(include) = config.include() else {
        return true;
    };

    let rel = relative_path(root, file);
    let includes = include
        .iter()
        .filter(|pattern| !pattern.starts_with('!'))
        .collect::<Vec<_>>();
    let excluded = include
        .iter()
        .filter_map(|pattern| pattern.strip_prefix('!'))
        .any(|pattern| match_pattern(&rel, pattern));

    !excluded
        && (includes.is_empty() || includes.iter().any(|pattern| match_pattern(&rel, pattern)))
}

fn scope_covers_file(scope: FileScope, file: &Path) -> bool {
    match scope {
        FileScope::JavaScriptLike => is_javascript_like_source(file),
        FileScope::AnySource => supports_import_scan(file),
    }
}

fn line_fix(
    content: &str,
    start_row: usize,
    end_row: usize,
    start_byte: usize,
    end_byte: usize,
    replacement: &str,
) -> Option<serde_json::Value> {
    let lines_start = line_start_byte(content, start_byte);
    let lines_end = line_end_byte(content, end_byte);
    let mut lines = content.get(lines_start..lines_end)?.to_string();
    lines.replace_range(
        (start_byte - lines_start)..(end_byte - lines_start),
        replacement,
    );

    Some(serde_json::json!({
        "startLine": start_row + 1,
        "endLine": end_row + 1,
        "replacement": lines,
    }))
}

fn single_line_fix(
    content: &str,
    start_row: usize,
    end_row: usize,
    start_byte: usize,
    end_byte: usize,
    replacement: &str,
) -> Option<serde_json::Value> {
    if start_row != end_row {
        return None;
    }
    line_fix(
        content,
        start_row,
        end_row,
        start_byte,
        end_byte,
        replacement,
    )
}

fn line_start_byte(content: &str, byte: usize) -> usize {
    content[..byte].rfind('\n').map_or(0, |index| index + 1)
}

fn line_end_byte(content: &str, byte: usize) -> usize {
    content[byte..]
        .find('\n')
        .map_or(content.len(), |index| byte + index)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project_root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    fn rule_config(yaml: &str) -> RuleConfig {
        serde_yaml::from_str(yaml).expect("valid rule config")
    }

    #[test]
    fn javascript_scope_skips_a_python_file() {
        let root = project_root();
        assert!(!rule_applies_to_file(
            FileScope::JavaScriptLike,
            &rule_config("error"),
            root,
            &root.join("src/a.py"),
        ));
    }

    #[test]
    fn any_source_scope_accepts_a_python_file() {
        let root = project_root();
        assert!(rule_applies_to_file(
            FileScope::AnySource,
            &rule_config("error"),
            root,
            &root.join("src/a.py"),
        ));
    }

    #[test]
    fn exclude_pattern_removes_a_file_matched_by_an_include_glob() {
        let root = project_root();
        assert!(!rule_applies_to_file(
            FileScope::AnySource,
            &rule_config("{include: [\"src/keep/**\", \"!src/keep/skip/**\"]}"),
            root,
            &root.join("src/keep/skip/a.ts"),
        ));
    }

    #[test]
    fn include_glob_accepts_a_matching_rust_file() {
        let root = project_root();
        assert!(rule_applies_to_file(
            FileScope::AnySource,
            &rule_config("{include: [\"src/**\"]}"),
            root,
            &root.join("src/a.rs"),
        ));
    }

    #[test]
    fn option_problems_are_empty_for_another_rule_with_a_non_array_value() {
        assert!(
            rule_option_problems("no-string-match", &rule_config("{statements: 5}")).is_empty()
        );
    }

    #[test]
    fn option_problems_are_empty_for_an_unknown_rule() {
        assert!(rule_option_problems("unknown-rule", &rule_config("error")).is_empty());
    }

    #[test]
    fn unknown_statement_keyword_is_one_option_problem() {
        assert_eq!(
            rule_option_problems(
                "blank-line-before-control-flow",
                &rule_config("{statements: [bogus]}")
            )
            .len(),
            1
        );
    }
}
