use super::grammar::{
    BooleanOperator, ComplexityGrammar, Construct, field_text, has_child_of_kind, operator_kind_is,
    text_of,
};
use tree_sitter::Node;

pub(super) struct Rust;

const FUNCTION_KINDS: &[&str] = &["function_item", "closure_expression"];

impl ComplexityGrammar for Rust {
    fn function_kinds(&self) -> &'static [&'static str] {
        FUNCTION_KINDS
    }

    fn construct(&self, node: Node<'_>, parent: Option<Node<'_>>) -> Construct {
        match node.kind() {
            "if_expression" if parent.is_some_and(|p| p.kind() == "else_clause") => {
                Construct::ChainedIf
            }
            "if_expression" | "match_expression" | "for_expression" | "while_expression"
            | "loop_expression" => Construct::Nesting,
            "else_clause" if has_child_of_kind(node, "block") => Construct::Increment,
            "let_declaration" if node.child_by_field_name("alternative").is_some() => {
                Construct::Nesting
            }
            "break_expression" | "continue_expression" if has_child_of_kind(node, "label") => {
                Construct::Increment
            }
            "match_pattern" if node.child_by_field_name("condition").is_some() => {
                Construct::Increment
            }
            _ => Construct::Plain,
        }
    }

    fn is_boolean_expression(&self, node: Node<'_>) -> bool {
        (node.kind() == "binary_expression" && operator_kind_is(node, &["&&", "||"]))
            || node.kind() == "let_chain"
    }

    fn boolean_operator(&self, token_kind: &str) -> Option<BooleanOperator> {
        match token_kind {
            "&&" => Some(BooleanOperator::And),
            "||" => Some(BooleanOperator::Or),
            _ => None,
        }
    }

    fn is_parenthesized(&self, node: Node<'_>) -> bool {
        node.kind() == "parenthesized_expression"
    }

    fn declared_name(&self, function: Node<'_>, source: &[u8]) -> Option<String> {
        field_text(function, "name", source)
    }

    fn assigned_name(&self, function: Node<'_>, source: &[u8]) -> Option<String> {
        let parent = function.parent()?;
        if parent.kind() != "let_declaration" {
            return None;
        }
        let pattern = parent.child_by_field_name("pattern")?;
        (pattern.kind() == "identifier").then(|| text_of(pattern, source))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::complexity::grammar::testing::{all_of_kind, parse};

    #[test]
    fn let_else_nests_and_a_plain_let_does_not() {
        let tree = parse(
            "a.rs",
            "fn f(a: Option<i32>) {\n    let Some(x) = a else { return; };\n    let y = 1;\n}\n",
        );
        let lets = all_of_kind(tree.root_node(), "let_declaration");
        assert_eq!(
            Rust.construct(lets[0], lets[0].parent()),
            Construct::Nesting
        );
        assert_eq!(Rust.construct(lets[1], lets[1].parent()), Construct::Plain);
    }

    #[test]
    fn labeled_break_increments_and_match_guard_increments() {
        let tree = parse(
            "a.rs",
            "fn f(a: i32) {\n    'outer: loop { break 'outer; }\n    match a { v if v > 1 => {} _ => {} }\n}\n",
        );
        let breaks = all_of_kind(tree.root_node(), "break_expression");
        let patterns = all_of_kind(tree.root_node(), "match_pattern");
        assert_eq!(
            Rust.construct(breaks[0], breaks[0].parent()),
            Construct::Increment
        );
        assert_eq!(
            Rust.construct(patterns[0], patterns[0].parent()),
            Construct::Increment
        );
    }

    #[test]
    fn let_binding_takes_the_name_of_a_closure() {
        let source = "fn f() { let g = || {}; }";
        let tree = parse("a.rs", source);
        let closures = all_of_kind(tree.root_node(), "closure_expression");
        assert_eq!(
            Rust.assigned_name(closures[0], source.as_bytes()),
            Some("g".to_string())
        );
    }
}
