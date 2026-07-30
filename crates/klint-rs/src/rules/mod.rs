mod builtin;
mod sonar;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::config::RuleConfig;
use crate::engine::{NodeIndex, Wants};
use crate::files::{is_javascript_like_source, match_pattern, relative_path};
use crate::output::Violation;

/// One file's indexed nodes, handed to every rule that applies to it. Rules
/// read the buckets they care about instead of each walking the tree.
#[derive(Clone, Copy)]
pub(crate) struct RuleScan<'a> {
    pub index: &'a NodeIndex<'a>,
    pub source: &'a [u8],
    pub content: &'a str,
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
const CHECKS: &[(&str, RuleCheck, RuleWants)] = &[
    ("no-string-match", builtin::run_no_string_match, |wants| {
        wants.calls = true;
    }),
    (
        "no-nested-template-literals",
        builtin::run_no_nested_template_literals,
        |wants| wants.templates = true,
    ),
    (
        "no-consecutive-array-push",
        builtin::run_no_consecutive_array_push,
        |wants| wants.containers = true,
    ),
    (
        "no-unguarded-json-parse",
        builtin::run_no_unguarded_json_parse,
        |wants| {
            wants.calls = true;
            wants.inside_try = true;
        },
    ),
    ("no-sync-in-async", builtin::run_no_sync_in_async, |wants| {
        wants.calls = true;
        wants.in_async = true;
    }),
    (
        "sonar/no-single-char-class",
        sonar::run_sonar_no_single_char_class,
        |wants| wants.regexes = true,
    ),
    ("sonar/prefer-at", sonar::run_sonar_prefer_at, |wants| {
        wants.subscripts = true
    }),
    (
        "sonar/prefer-string-replaceall",
        sonar::run_sonar_prefer_string_replaceall,
        |wants| wants.calls = true,
    ),
    (
        "sonar/prefer-string-raw",
        sonar::run_sonar_prefer_string_raw,
        |wants| wants.strings = true,
    ),
    (
        "sonar/prefer-string-raw-regexp",
        sonar::run_sonar_prefer_string_raw_regexp,
        |wants| wants.new_expressions = true,
    ),
    (
        "sonar/prefer-nullish-coalescing-assign",
        sonar::run_sonar_prefer_nullish_coalescing_assign,
        |wants| wants.if_statements = true,
    ),
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
}

impl RulePass {
    pub(crate) fn covers(&self, file_index: usize) -> bool {
        self.applies.get(file_index).copied().unwrap_or(false)
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
pub(crate) fn plan_rule_passes(
    rules: &BTreeMap<String, RuleConfig>,
    files: &[PathBuf],
    root: &Path,
) -> Vec<RulePass> {
    CHECKS
        .iter()
        .filter_map(|(rule, check, wants)| {
            let config = rules.get(*rule)?;
            let severity = config.severity();
            if severity == "off" {
                return None;
            }
            Some(RulePass {
                rule,
                check: *check,
                wants: *wants,
                severity: severity.to_string(),
                applies: files
                    .iter()
                    .map(|file| rule_applies_to_file(config, root, file))
                    .collect(),
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

fn rule_applies_to_file(config: &RuleConfig, root: &Path, file: &Path) -> bool {
    if !is_javascript_like_source(file) {
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

fn line_fix(
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

    let line_start = line_start_byte(content, start_byte);
    let line_end = line_end_byte(content, start_byte);
    let mut line = content.get(line_start..line_end)?.to_string();
    line.replace_range(
        (start_byte - line_start)..(end_byte - line_start),
        replacement,
    );

    Some(serde_json::json!({
        "startLine": start_row + 1,
        "endLine": end_row + 1,
        "replacement": line,
    }))
}

fn line_start_byte(content: &str, byte: usize) -> usize {
    content[..byte].rfind('\n').map_or(0, |index| index + 1)
}

fn line_end_byte(content: &str, byte: usize) -> usize {
    content[byte..]
        .find('\n')
        .map_or(content.len(), |index| byte + index)
}
