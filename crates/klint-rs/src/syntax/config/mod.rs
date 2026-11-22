mod json;
mod toml;
mod yaml;

use crate::files::match_component;
use std::ops::Range;
use std::path::Path;
use tree_sitter::{Language, Node, Parser};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfigEntry {
    pub path: Vec<String>,
    pub key: Range<usize>,
    pub value: Option<String>,
}

#[derive(Clone, Copy)]
enum Format {
    Json,
    Yaml,
    Toml,
}

pub(crate) fn is_config_path(path: &Path) -> bool {
    format_of(path).is_some()
}

pub(crate) fn config_entries(path: &Path, content: &str) -> Option<Vec<ConfigEntry>> {
    let format = format_of(path)?;
    let mut parser = Parser::new();
    parser.set_language(&language(format)).ok()?;
    let tree = parser.parse(content, None)?;
    let root = tree.root_node();

    if root.has_error() {
        return None;
    }

    let mut entries = Entries {
        source: content.as_bytes(),
        list: Vec::new(),
    };

    match format {
        Format::Json => json::collect(root, &mut entries),
        Format::Yaml => yaml::collect(root, &mut entries),
        Format::Toml => toml::collect(root, &mut entries),
    }

    Some(entries.list)
}

fn format_of(path: &Path) -> Option<Format> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("json" | "jsonc") => Some(Format::Json),
        Some("yaml" | "yml") => Some(Format::Yaml),
        Some("toml") => Some(Format::Toml),
        _ => None,
    }
}

fn language(format: Format) -> Language {
    match format {
        Format::Json => tree_sitter_json::LANGUAGE.into(),
        Format::Yaml => tree_sitter_yaml::LANGUAGE.into(),
        Format::Toml => tree_sitter_toml_ng::LANGUAGE.into(),
    }
}

struct Entries<'s> {
    source: &'s [u8],
    list: Vec<ConfigEntry>,
}

impl Entries<'_> {
    fn text(&self, node: Node<'_>) -> String {
        node.utf8_text(self.source).unwrap_or_default().to_string()
    }

    fn unquoted(&self, node: Node<'_>) -> String {
        unquote(&self.text(node)).to_string()
    }

    fn push(&mut self, path: &[String], key: Node<'_>, value: Option<String>) {
        self.list.push(ConfigEntry {
            path: path.to_vec(),
            key: key.byte_range(),
            value,
        });
    }
}

fn unquote(text: &str) -> &str {
    ["\"\"\"", "'''", "\"", "'"]
        .iter()
        .find_map(|quote| text.strip_prefix(quote)?.strip_suffix(quote))
        .unwrap_or(text)
}

fn joined(path: &[String], segment: String) -> Vec<String> {
    let mut joined = path.to_vec();
    joined.push(segment);
    joined
}

fn named_children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

pub(crate) fn path_matches(pattern: &[String], path: &[String]) -> bool {
    match pattern.split_first() {
        None => path.is_empty(),
        Some((first, rest)) if first == "**" => {
            (1..=path.len()).any(|taken| path_matches(rest, &path[taken..]))
        }
        Some((first, rest)) => path.split_first().is_some_and(|(segment, tail)| {
            match_component(first, segment) && path_matches(rest, tail)
        }),
    }
}

pub(crate) fn path_starts_with(path: &[String], pattern: &[String]) -> bool {
    (1..path.len()).any(|length| path_matches(pattern, &path[..length]))
}

#[cfg(test)]
mod tests;
