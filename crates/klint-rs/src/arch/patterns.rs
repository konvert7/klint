use super::layers::resolve_layer_mask;
use super::*;
use crate::files::{normalize_path, relative_path};
use crate::output::Violation;
use crate::syntax::{is_jsx_path, scan_jsx_elements_from_tree};
use std::path::{Path, PathBuf};
use tree_sitter::Node;

/// A `forbidden` or `singleton` rule bound to the files it covers. Exactly one
/// of `matcher`/`jsx_targets` is set — jsx-element rules take precedence in the
/// config, and a rule with neither never becomes a pass.
pub(super) struct PatternPass {
    rule_name: &'static str,
    scope: Vec<bool>,
    matcher: Option<LineMatcher>,
    jsx_targets: Option<Vec<String>>,
    message: String,
    severity: String,
}

pub(super) fn plan_forbidden_passes(
    arch: &ArchConfig,
    files: &[PathBuf],
    root: &Path,
) -> Vec<PatternPass> {
    let Some(rules) = &arch.forbidden else {
        return Vec::new();
    };

    rules
        .iter()
        .filter_map(|rule| {
            let scope = resolve_layer_mask(&rule.in_scope, arch.layers.as_ref(), root, files);
            build_pass(
                "arch/forbidden",
                scope,
                rule.jsx_element.as_ref(),
                rule.pattern.as_deref(),
                &rule.message,
                rule.severity.as_deref(),
            )
        })
        .collect()
}

pub(super) fn plan_singleton_passes(
    arch: &ArchConfig,
    files: &[PathBuf],
    root: &Path,
) -> Vec<PatternPass> {
    let Some(rules) = &arch.singleton else {
        return Vec::new();
    };

    rules
        .iter()
        .filter_map(|rule| {
            let only_file = normalize_path(&root.join(&rule.only));
            let mut scope = match &rule.in_scope {
                Some(in_scope) => resolve_layer_mask(in_scope, arch.layers.as_ref(), root, files),
                None => vec![true; files.len()],
            };
            for (index, file) in files.iter().enumerate() {
                if file == &only_file {
                    scope[index] = false;
                }
            }
            build_pass(
                "arch/singleton",
                scope,
                rule.jsx_element.as_ref(),
                rule.pattern.as_deref(),
                &rule.message,
                rule.severity.as_deref(),
            )
        })
        .collect()
}

fn build_pass(
    rule_name: &'static str,
    scope: Vec<bool>,
    jsx_element: Option<&StringOrVec>,
    pattern: Option<&str>,
    message: &str,
    severity: Option<&str>,
) -> Option<PatternPass> {
    let (matcher, jsx_targets) = match (jsx_element, pattern) {
        (Some(targets), _) => (None, Some(targets.items())),
        (None, Some(pattern)) => (Some(LineMatcher::build(pattern)), None),
        (None, None) => return None,
    };

    Some(PatternPass {
        rule_name,
        scope,
        matcher,
        jsx_targets,
        message: message.to_string(),
        severity: severity.unwrap_or("error").to_string(),
    })
}

impl PatternPass {
    pub(super) fn covers(&self, file_index: usize, file: &Path) -> bool {
        if !self.scope.get(file_index).copied().unwrap_or(false) {
            return false;
        }
        // Only jsx-path files can hold a jsx node, so skip the rest rather
        // than asking for a parse that cannot match.
        self.jsx_targets.is_none() || is_jsx_path(file)
    }

    pub(super) fn needs_tree(&self) -> bool {
        self.jsx_targets.is_some()
    }

    pub(super) fn scan(
        &self,
        file: &Path,
        root: &Path,
        tree_root: Option<Node<'_>>,
        source: &[u8],
        content: &str,
    ) -> Vec<Violation> {
        match (&self.jsx_targets, &self.matcher) {
            (Some(targets), _) => self.scan_elements(file, root, tree_root, source, targets),
            (None, Some(matcher)) => self.scan_lines(file, root, content, matcher),
            (None, None) => Vec::new(),
        }
    }

    fn scan_elements(
        &self,
        file: &Path,
        root: &Path,
        tree_root: Option<Node<'_>>,
        source: &[u8],
        targets: &[String],
    ) -> Vec<Violation> {
        let Some(tree_root) = tree_root else {
            return Vec::new();
        };
        scan_jsx_elements_from_tree(tree_root, source)
            .into_iter()
            .filter(|element| targets.contains(&element.tag_name))
            .map(|element| self.violation(file, root, element.line))
            .collect()
    }

    fn scan_lines(
        &self,
        file: &Path,
        root: &Path,
        content: &str,
        matcher: &LineMatcher,
    ) -> Vec<Violation> {
        content
            .lines()
            .enumerate()
            .filter(|(_, line)| matcher.is_match(line))
            .map(|(index, _)| self.violation(file, root, index + 1))
            .collect()
    }

    fn violation(&self, file: &Path, root: &Path, line: usize) -> Violation {
        Violation {
            file: relative_path(root, file),
            line,
            rule: self.rule_name.to_string(),
            message: self.message.clone(),
            severity: self.severity.clone(),
            fix: None,
        }
    }
}

const REGEX_PREFIX: &str = "re:";

enum LineMatcher {
    Literal(String),
    Regex(regex::Regex),
}

impl LineMatcher {
    fn build(pattern: &str) -> Self {
        let Some(source) = pattern.strip_prefix(REGEX_PREFIX) else {
            return Self::Literal(pattern.to_string());
        };
        match regex::Regex::new(source) {
            Ok(regex) => Self::Regex(regex),
            Err(error) => {
                eprintln!("klint: invalid regex in arch pattern {pattern:?}: {error}");
                Self::Regex(regex::Regex::new("[^\\s\\S]").expect("never-matching regex is valid"))
            }
        }
    }

    fn is_match(&self, line: &str) -> bool {
        match self {
            Self::Literal(pattern) => line.contains(pattern.as_str()),
            Self::Regex(regex) => regex.is_match(line),
        }
    }
}
