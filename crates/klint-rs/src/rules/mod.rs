mod builtin;
mod sonar;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tree_sitter::Node;

use crate::config::RuleConfig;
use crate::files::{is_javascript_like_source, match_pattern, relative_path};
use crate::output::Violation;
use crate::syntax::TreeCache;

/// The inputs every rule check shares, bundled so a rule signature carries
/// only what makes that rule different.
#[derive(Clone, Copy)]
pub(super) struct RuleRun<'a> {
    files: &'a [PathBuf],
    file_contents: &'a BTreeMap<PathBuf, String>,
    tree_cache: &'a TreeCache,
    root: &'a Path,
}

/// What a rule has to say about one scanned record. The file, rule id, and
/// severity are the same for every record a check produces, so [`RuleRun::check`]
/// fills those in.
pub(super) struct Report {
    pub line: usize,
    pub message: String,
    pub fix: Option<serde_json::Value>,
}

impl RuleRun<'_> {
    /// Walks the files this rule applies to, parses each once through the
    /// shared cache, and turns every scanned record into a violation.
    fn check<R>(
        self,
        rule: &str,
        config: &RuleConfig,
        scan: impl Fn(Node<'_>, &[u8]) -> Vec<R>,
        report: impl Fn(&R, &str) -> Report,
    ) -> Vec<Violation> {
        let severity = config.severity();
        if severity == "off" {
            return Vec::new();
        }

        let mut violations = Vec::new();
        for file in self.files {
            if !rule_applies_to_file(config, self.root, file) {
                continue;
            }
            let Some(content) = self.file_contents.get(file) else {
                continue;
            };
            let Some(tree) = self.tree_cache.get_or_parse(file, content) else {
                continue;
            };

            for record in scan(tree.root_node(), content.as_bytes()) {
                let reported = report(&record, content);
                violations.push(Violation {
                    file: relative_path(self.root, file),
                    line: reported.line,
                    rule: rule.to_string(),
                    message: reported.message,
                    severity: severity.to_string(),
                    fix: reported.fix,
                });
            }
        }
        violations
    }
}

type RuleCheck = fn(&RuleConfig, RuleRun<'_>) -> Vec<Violation>;

/// Every rule the Rust engine supports, in the order their violations are
/// emitted.
const CHECKS: &[(&str, RuleCheck)] = &[
    ("no-string-match", builtin::run_no_string_match),
    (
        "no-nested-template-literals",
        builtin::run_no_nested_template_literals,
    ),
    (
        "no-consecutive-array-push",
        builtin::run_no_consecutive_array_push,
    ),
    (
        "no-unguarded-json-parse",
        builtin::run_no_unguarded_json_parse,
    ),
    ("no-sync-in-async", builtin::run_no_sync_in_async),
    (
        "sonar/no-single-char-class",
        sonar::run_sonar_no_single_char_class,
    ),
    ("sonar/prefer-at", sonar::run_sonar_prefer_at),
    (
        "sonar/prefer-string-replaceall",
        sonar::run_sonar_prefer_string_replaceall,
    ),
    (
        "sonar/prefer-string-raw",
        sonar::run_sonar_prefer_string_raw,
    ),
    (
        "sonar/prefer-string-raw-regexp",
        sonar::run_sonar_prefer_string_raw_regexp,
    ),
    (
        "sonar/prefer-nullish-coalescing-assign",
        sonar::run_sonar_prefer_nullish_coalescing_assign,
    ),
];

/// Runs every configured rule as its own scoped thread — each rule already
/// walks the full file list independently, so this just lets those
/// independent walks happen concurrently instead of one after another. Every
/// thread is spawned before any is joined, and results are joined back in
/// [`CHECKS`] order, so output ordering is unaffected by completion order.
pub(crate) fn run_supported_rules(
    rules: &BTreeMap<String, RuleConfig>,
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    tree_cache: &TreeCache,
    root: &Path,
) -> Vec<Violation> {
    let run = RuleRun {
        files,
        file_contents,
        tree_cache,
        root,
    };

    std::thread::scope(|scope| {
        CHECKS
            .iter()
            .filter_map(|(rule, check)| rules.get(*rule).map(|config| (config, check)))
            .map(|(config, check)| scope.spawn(move || check(config, run)))
            .collect::<Vec<_>>()
            .into_iter()
            .flat_map(|handle| handle.join().expect("rule check thread panicked"))
            .collect()
    })
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
