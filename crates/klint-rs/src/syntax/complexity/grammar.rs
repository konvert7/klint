use super::csharp::CSharp;
use super::javascript::JavaScript;
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

pub(super) trait ComplexityGrammar: Sync {
    fn function_kinds(&self) -> &'static [&'static str];
    fn scores_leaf_kind(&self, _kind: &str) -> bool {
        false
    }
    fn construct(&self, node: Node<'_>, parent: Option<Node<'_>>) -> Construct;
    fn is_boolean_expression(&self, node: Node<'_>) -> bool;
    fn boolean_operator(&self, token_kind: &str) -> Option<BooleanOperator>;
    fn boolean_run_points(&self, _node: Node<'_>, _parent: Option<Node<'_>>) -> Option<u32> {
        None
    }
    fn declared_name(&self, function: Node<'_>, source: &[u8]) -> Option<String>;
    fn assigned_name(&self, function: Node<'_>, source: &[u8]) -> Option<String>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Construct {
    Plain,
    Function,
    Nesting,
    ChainedIf,
    Increment,
    LoopElse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BooleanOperator {
    And,
    Or,
    Nullish,
}

pub(super) fn complexity_grammar_for(language: SourceLanguage) -> &'static dyn ComplexityGrammar {
    match language {
        SourceLanguage::JavaScriptLike => &JAVASCRIPT,
        SourceLanguage::Python => &PYTHON,
        SourceLanguage::Swift => &SWIFT,
        SourceLanguage::Rust => &RUST,
        SourceLanguage::CSharp => &CSHARP,
    }
}

pub(super) fn text_of(node: Node<'_>, source: &[u8]) -> String {
    node.utf8_text(source)
        .map(str::to_owned)
        .unwrap_or_default()
}

pub(super) fn field_text(node: Node<'_>, field: &str, source: &[u8]) -> Option<String> {
    node.child_by_field_name(field)
        .map(|child| text_of(child, source))
}

pub(super) fn has_child_of_kind(node: Node<'_>, kind: &str) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|child| child.kind() == kind)
}

pub(super) fn first_descendant_of_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    if node.kind() == kind {
        return Some(node);
    }
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find_map(|child| first_descendant_of_kind(child, kind))
}

pub(super) fn is_field_of(parent: Node<'_>, field: &str, node: Node<'_>) -> bool {
    parent
        .child_by_field_name(field)
        .is_some_and(|child| child.id() == node.id())
}

pub(super) fn operator_kind_is(node: Node<'_>, kinds: &[&str]) -> bool {
    node.child_by_field_name("operator")
        .is_some_and(|operator| kinds.contains(&operator.kind()))
}

#[cfg(test)]
pub(super) mod testing {
    use crate::syntax::parse_source;
    use std::path::Path;
    use tree_sitter::{Node, Tree};

    pub(in crate::syntax::complexity) fn parse(path: &str, source: &str) -> Tree {
        parse_source(Path::new(path), source).expect("snippet parses")
    }

    pub(in crate::syntax::complexity) fn all_of_kind<'a>(
        root: Node<'a>,
        kind: &str,
    ) -> Vec<Node<'a>> {
        let mut found = Vec::new();
        let mut cursor = root.walk();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            if node.kind() == kind {
                found.push(node);
            }
            let children: Vec<Node<'a>> = node.children(&mut cursor).collect();
            stack.extend(children.into_iter().rev());
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::complexity::grammar::testing::{all_of_kind, parse};

    fn first_of_kind<'a>(root: Node<'a>, kind: &str) -> Node<'a> {
        first_descendant_of_kind(root, kind).expect("node of that kind")
    }

    #[test]
    fn field_text_reads_the_named_child_and_is_absent_without_it() {
        let source = "function f() {}\nconst g = () => 1;";
        let tree = parse("a.ts", source);
        let declaration = first_of_kind(tree.root_node(), "function_declaration");
        let arrow = first_of_kind(tree.root_node(), "arrow_function");
        assert_eq!(
            field_text(declaration, "name", source.as_bytes()),
            Some("f".to_string())
        );
        assert_eq!(field_text(arrow, "name", source.as_bytes()), None);
    }

    #[test]
    fn has_child_of_kind_sees_a_label_only_when_present() {
        let labeled = parse("a.ts", "outer: for (;;) { break outer; }");
        let plain = parse("a.ts", "for (;;) { break; }");
        let labeled_break = first_of_kind(labeled.root_node(), "break_statement");
        let plain_break = first_of_kind(plain.root_node(), "break_statement");
        assert!(has_child_of_kind(labeled_break, "statement_identifier"));
        assert!(!has_child_of_kind(plain_break, "statement_identifier"));
    }

    #[test]
    fn operator_kind_is_true_only_for_listed_operators() {
        let sum_tree = parse("a.ts", "a + b;");
        let coalesce_tree = parse("a.ts", "a ?? b;");
        let sum = first_of_kind(sum_tree.root_node(), "binary_expression");
        let coalesce = first_of_kind(coalesce_tree.root_node(), "binary_expression");
        let listed = ["&&", "||", "??"];
        assert!(!operator_kind_is(sum, &listed));
        assert!(operator_kind_is(coalesce, &listed));
    }

    #[test]
    fn first_descendant_includes_the_node_itself() {
        let tree = parse("a.ts", "function f() {}");
        let declaration = first_of_kind(tree.root_node(), "function_declaration");
        assert_eq!(
            first_descendant_of_kind(declaration, "function_declaration").map(|node| node.id()),
            Some(declaration.id())
        );
    }

    #[test]
    fn all_of_kind_lists_every_match_in_source_order() {
        let tree = parse("a.ts", "f(); g(); if (a) { h(); }");
        assert_eq!(all_of_kind(tree.root_node(), "call_expression").len(), 3);
    }
}
