use super::layers::resolve_text_mask;
use super::patterns::LineMatcher;
use super::{ArchConfig, StringOrVec};
use crate::files::relative_path;
use crate::output::{Span, Violation};
use crate::syntax::{ConfigEntry, is_config_path, path_matches, path_starts_with};
use std::path::{Path, PathBuf};

enum Check {
    Forbidden,
    Required,
}

pub(super) struct KeyPass {
    rule_name: &'static str,
    check: Check,
    scope: Vec<bool>,
    key: Vec<String>,
    matcher: Option<LineMatcher>,
    message: String,
    severity: String,
}

struct KeyRule<'r> {
    key: &'r str,
    pattern: Option<&'r str>,
    in_scope: &'r StringOrVec,
    message: &'r str,
    severity: Option<&'r str>,
}

pub(super) fn plan_key_passes(arch: &ArchConfig, files: &[PathBuf], root: &Path) -> Vec<KeyPass> {
    let forbidden = arch.forbidden.iter().flatten().filter_map(|rule| {
        let rule_key = KeyRule {
            key: rule.key.as_deref()?,
            pattern: rule.pattern.as_deref(),
            in_scope: &rule.in_scope,
            message: &rule.message,
            severity: rule.severity.as_deref(),
        };
        Some(("arch/forbidden", Check::Forbidden, rule_key))
    });
    let required = arch.required.iter().flatten().map(|rule| {
        let rule_key = KeyRule {
            key: &rule.key,
            pattern: rule.pattern.as_deref(),
            in_scope: &rule.in_scope,
            message: &rule.message,
            severity: rule.severity.as_deref(),
        };
        ("arch/required", Check::Required, rule_key)
    });

    forbidden
        .chain(required)
        .map(|(rule_name, check, rule)| KeyPass {
            rule_name,
            check,
            scope: resolve_text_mask(rule.in_scope, arch.layers.as_ref(), root, files),
            key: rule.key.split('.').map(str::to_string).collect(),
            matcher: rule.pattern.map(LineMatcher::build),
            message: rule.message.to_string(),
            severity: rule.severity.unwrap_or("error").to_string(),
        })
        .collect()
}

impl KeyPass {
    pub(super) fn covers(&self, file_index: usize, file: &Path) -> bool {
        self.scope.get(file_index).copied().unwrap_or(false) && is_config_path(file)
    }

    pub(super) fn scan(
        &self,
        file: &Path,
        root: &Path,
        content: &str,
        entries: &[ConfigEntry],
    ) -> Vec<Violation> {
        let found = entries_at(entries, &self.key);
        let flagged: Vec<&ConfigEntry> = match self.check {
            Check::Forbidden => found
                .into_iter()
                .filter(|entry| self.value_matches(entry))
                .collect(),
            Check::Required if found.is_empty() => {
                return vec![self.violation(file, root, 1, None)];
            }
            Check::Required => found
                .into_iter()
                .filter(|entry| !self.value_matches(entry))
                .collect(),
        };

        flagged
            .into_iter()
            .map(|entry| {
                let line = content[..entry.key.start].matches('\n').count() + 1;
                let span = Span::within(content, entry.key.clone());
                self.violation(file, root, line, span)
            })
            .collect()
    }

    fn value_matches(&self, entry: &ConfigEntry) -> bool {
        self.matcher.as_ref().is_none_or(|matcher| {
            entry
                .value
                .as_deref()
                .is_some_and(|value| matcher.matches_value(value))
        })
    }

    fn violation(&self, file: &Path, root: &Path, line: usize, span: Option<Span>) -> Violation {
        Violation {
            file: relative_path(root, file),
            line,
            rule: self.rule_name.to_string(),
            message: self.message.clone(),
            severity: self.severity.clone(),
            fix: None,
            span,
        }
    }
}

fn entries_at<'e>(entries: &'e [ConfigEntry], key: &[String]) -> Vec<&'e ConfigEntry> {
    let exact: Vec<&ConfigEntry> = entries
        .iter()
        .filter(|entry| path_matches(key, &entry.path))
        .collect();

    if !exact.is_empty() {
        return exact;
    }

    entries
        .iter()
        .find(|entry| path_starts_with(&entry.path, key))
        .into_iter()
        .collect()
}
