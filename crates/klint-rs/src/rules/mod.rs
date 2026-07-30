mod builtin;
mod sonar;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::config::RuleConfig;
use crate::files::{is_javascript_like_source, match_pattern, relative_path};
use crate::output::Violation;
use crate::syntax::TreeCache;
use builtin::{
    run_no_consecutive_array_push, run_no_nested_template_literals, run_no_string_match,
    run_no_sync_in_async, run_no_unguarded_json_parse,
};
use sonar::{
    run_sonar_no_single_char_class, run_sonar_prefer_at,
    run_sonar_prefer_nullish_coalescing_assign, run_sonar_prefer_string_raw,
    run_sonar_prefer_string_raw_regexp, run_sonar_prefer_string_replaceall,
};

/// Runs every configured rule as its own scoped thread — each rule already
/// walks the full file list independently, so this just lets those
/// independent walks happen concurrently instead of one after another.
/// Results are joined back in the same fixed order the rules are checked
/// in below, so output ordering is unaffected by thread completion order.
pub(crate) fn run_supported_rules(
    rules: &BTreeMap<String, RuleConfig>,
    files: &[PathBuf],
    file_contents: &BTreeMap<PathBuf, String>,
    tree_cache: &TreeCache,
    root: &Path,
) -> Vec<Violation> {
    std::thread::scope(|scope| {
        let mut handles = Vec::new();

        if let Some(config) = rules.get("no-string-match") {
            handles.push(
                scope.spawn(|| run_no_string_match(config, files, file_contents, tree_cache, root)),
            );
        }
        if let Some(config) = rules.get("no-nested-template-literals") {
            handles.push(scope.spawn(|| {
                run_no_nested_template_literals(config, files, file_contents, tree_cache, root)
            }));
        }
        if let Some(config) = rules.get("no-consecutive-array-push") {
            handles.push(scope.spawn(|| {
                run_no_consecutive_array_push(config, files, file_contents, tree_cache, root)
            }));
        }
        if let Some(config) = rules.get("no-unguarded-json-parse") {
            handles.push(scope.spawn(|| {
                run_no_unguarded_json_parse(config, files, file_contents, tree_cache, root)
            }));
        }
        if let Some(config) = rules.get("no-sync-in-async") {
            handles
                .push(scope.spawn(|| {
                    run_no_sync_in_async(config, files, file_contents, tree_cache, root)
                }));
        }
        if let Some(config) = rules.get("sonar/no-single-char-class") {
            handles.push(scope.spawn(|| {
                run_sonar_no_single_char_class(config, files, file_contents, tree_cache, root)
            }));
        }
        if let Some(config) = rules.get("sonar/prefer-at") {
            handles.push(
                scope.spawn(|| run_sonar_prefer_at(config, files, file_contents, tree_cache, root)),
            );
        }
        if let Some(config) = rules.get("sonar/prefer-string-replaceall") {
            handles.push(scope.spawn(|| {
                run_sonar_prefer_string_replaceall(config, files, file_contents, tree_cache, root)
            }));
        }
        if let Some(config) = rules.get("sonar/prefer-string-raw") {
            handles.push(scope.spawn(|| {
                run_sonar_prefer_string_raw(config, files, file_contents, tree_cache, root)
            }));
        }
        if let Some(config) = rules.get("sonar/prefer-string-raw-regexp") {
            handles.push(scope.spawn(|| {
                run_sonar_prefer_string_raw_regexp(config, files, file_contents, tree_cache, root)
            }));
        }
        if let Some(config) = rules.get("sonar/prefer-nullish-coalescing-assign") {
            handles.push(scope.spawn(|| {
                run_sonar_prefer_nullish_coalescing_assign(
                    config,
                    files,
                    file_contents,
                    tree_cache,
                    root,
                )
            }));
        }

        handles
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
