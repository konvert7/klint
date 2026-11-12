use super::csharp::CSharp;
use super::javascript::JavaScript;
use super::keyword::Keyword;
use super::python::Python;
use super::rust::Rust;
use super::swift::Swift;
use crate::syntax::SourceLanguage;
use tree_sitter::Node;

static JAVASCRIPT: JavaScript = JavaScript;
static PYTHON: Python = Python;
static SWIFT: Swift = Swift;
static RUST: Rust = Rust;
static CSHARP: CSharp = CSharp;

pub(super) trait Grammar: Sync {
    fn containers(&self) -> &'static [&'static str];
    fn is_body_member(&self, child: Node<'_>, field: Option<&str>) -> bool;
    fn classify<'a>(&self, member: Node<'a>) -> Role<'a>;
    fn assigned_names(&self, statement: Node<'_>, source: &[u8]) -> Vec<String>;
    fn is_condition_name(&self, node: Node<'_>) -> bool;
    fn is_directive_line(&self, _line: &str) -> bool {
        false
    }
}

pub(super) enum Role<'a> {
    Comment,
    Prefix,
    Statement(Option<Target<'a>>),
}

pub(super) struct Target<'a> {
    pub keyword: Keyword,
    pub written: &'static str,
    pub ends_with_block: bool,
    pub conditions: Vec<Node<'a>>,
}

pub(super) fn grammar_for(language: SourceLanguage) -> &'static dyn Grammar {
    match language {
        SourceLanguage::JavaScriptLike => &JAVASCRIPT,
        SourceLanguage::Python => &PYTHON,
        SourceLanguage::Swift => &SWIFT,
        SourceLanguage::Rust => &RUST,
        SourceLanguage::CSharp => &CSHARP,
    }
}

pub(crate) fn is_statement_container(language: SourceLanguage, kind: &str) -> bool {
    grammar_for(language).containers().contains(&kind)
}

pub(super) fn named_children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

pub(super) fn field_nodes<'a>(node: Node<'a>, field: &str) -> Vec<Node<'a>> {
    let mut cursor = node.walk();
    node.children_by_field_name(field, &mut cursor).collect()
}

pub(super) fn node_text(node: Node<'_>, source: &[u8]) -> String {
    node.utf8_text(source)
        .map(str::to_owned)
        .unwrap_or_default()
}

pub(super) fn is_field_of(parent: Node<'_>, field: &str, node: Node<'_>) -> bool {
    parent
        .child_by_field_name(field)
        .is_some_and(|child| child.id() == node.id())
}

pub(super) fn block_target<'a>(keyword: Keyword, written: &'static str) -> Target<'a> {
    Target {
        keyword,
        written,
        ends_with_block: true,
        conditions: Vec::new(),
    }
}

pub(super) fn body_target<'a>(
    node: Node<'a>,
    keyword: Keyword,
    written: &'static str,
    block_kind: &str,
) -> Target<'a> {
    let ends_with_block = node
        .child_by_field_name("body")
        .is_some_and(|body| body.kind() == block_kind);
    Target {
        keyword,
        written,
        ends_with_block,
        conditions: Vec::new(),
    }
}

pub(super) fn descendants_of_kind<'a>(node: Node<'a>, kinds: &[&str]) -> Vec<Node<'a>> {
    let mut found = Vec::new();
    collect_descendants_of_kind(node, kinds, &mut found);
    found
}

fn collect_descendants_of_kind<'a>(node: Node<'a>, kinds: &[&str], found: &mut Vec<Node<'a>>) {
    if kinds.contains(&node.kind()) {
        found.push(node);
    }

    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        collect_descendants_of_kind(child, kinds, found);
    }
}

pub(super) fn ends_with_braced_body(
    node: Node<'_>,
    block_kind: &str,
    else_kind: Option<&str>,
) -> bool {
    match node.child_by_field_name("alternative") {
        Some(alternative) => ends_with_else_body(alternative, block_kind, else_kind),
        None => node
            .child_by_field_name("consequence")
            .is_some_and(|consequence| consequence.kind() == block_kind),
    }
}

fn ends_with_else_body(alternative: Node<'_>, block_kind: &str, else_kind: Option<&str>) -> bool {
    let body = match else_kind {
        Some(kind) if alternative.kind() == kind => last_named_non_comment(alternative),
        _ => Some(alternative),
    };
    match body {
        Some(body) if body.kind() == "if_statement" => {
            ends_with_braced_body(body, block_kind, else_kind)
        }
        Some(body) => body.kind() == block_kind,
        None => false,
    }
}

fn last_named_non_comment(node: Node<'_>) -> Option<Node<'_>> {
    named_children(node)
        .into_iter()
        .rev()
        .find(|child| child.kind() != "comment")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::{SourceLanguage, parse_source};
    use std::path::Path;

    #[test]
    fn javascript_switch_case_is_a_statement_container() {
        assert!(is_statement_container(
            SourceLanguage::JavaScriptLike,
            "switch_case"
        ));
    }

    #[test]
    fn javascript_block_is_not_a_statement_container() {
        assert!(!is_statement_container(
            SourceLanguage::JavaScriptLike,
            "block"
        ));
    }

    #[test]
    fn python_block_is_a_statement_container() {
        assert!(is_statement_container(SourceLanguage::Python, "block"));
    }

    #[test]
    fn rust_statement_block_is_not_a_statement_container() {
        assert!(!is_statement_container(
            SourceLanguage::Rust,
            "statement_block"
        ));
    }

    #[test]
    fn csharp_switch_section_is_a_statement_container() {
        assert!(is_statement_container(
            SourceLanguage::CSharp,
            "switch_section"
        ));
    }

    #[test]
    fn swift_statements_is_a_statement_container() {
        assert!(is_statement_container(SourceLanguage::Swift, "statements"));
    }

    fn if_statement_ends_with_braced_body(source: &str) -> bool {
        let tree = parse_source(Path::new("a.ts"), source).expect("source parses");
        let statement = tree
            .root_node()
            .named_child(0)
            .expect("one top-level statement");
        ends_with_braced_body(statement, "statement_block", Some("else_clause"))
    }

    #[test]
    fn if_with_braced_body_ends_with_block() {
        assert!(if_statement_ends_with_braced_body("if (a) { b(); }"));
    }

    #[test]
    fn if_with_braceless_body_does_not_end_with_block() {
        assert!(!if_statement_ends_with_braced_body("if (a) b();"));
    }

    #[test]
    fn if_with_braceless_else_does_not_end_with_block() {
        assert!(!if_statement_ends_with_braced_body(
            "if (a) { b(); } else c();"
        ));
    }

    #[test]
    fn else_if_chain_ending_in_a_braced_body_ends_with_block() {
        assert!(if_statement_ends_with_braced_body(
            "if (a) { } else if (c) { }"
        ));
    }

    #[test]
    fn if_whose_body_is_a_for_loop_does_not_end_with_block() {
        assert!(!if_statement_ends_with_braced_body("if (a) for (;;) { }"));
    }
}
