use super::grammar::{
    BooleanOperator, ComplexityGrammar, Construct, field_text, has_child_of_kind, is_in_any_field,
    operator_kind_is, text_of,
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
            "block" if is_let_else_block(node, parent) => Construct::Nesting,
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

    fn is_head(&self, node: Node<'_>, parent: Node<'_>) -> bool {
        is_in_any_field(parent, head_fields(parent.kind()), node)
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

fn head_fields(kind: &str) -> &'static [&'static str] {
    match kind {
        "if_expression" | "while_expression" => &["condition"],
        "match_expression" => &["value"],
        "for_expression" => &["pattern", "value"],
        _ => &[],
    }
}

fn is_let_else_block(block: Node<'_>, parent: Option<Node<'_>>) -> bool {
    parent
        .and_then(|p| p.child_by_field_name("alternative"))
        .is_some_and(|alternative| alternative.id() == block.id())
        && parent.is_some_and(|p| p.kind() == "let_declaration")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::complexity::grammar::testing::{all_of_kind, parse};

    #[test]
    fn let_else_block_nests_and_the_declaration_does_not() {
        let tree = parse(
            "a.rs",
            "fn f(a: Option<i32>) {\n    let Some(x) = a else { return; };\n    let y = 1;\n}\n",
        );
        let lets = all_of_kind(tree.root_node(), "let_declaration");
        let else_block = lets[0]
            .child_by_field_name("alternative")
            .expect("let-else has an alternative block");
        assert_eq!(
            Rust.construct(else_block, Some(lets[0])),
            Construct::Nesting
        );
        assert_eq!(Rust.construct(lets[0], lets[0].parent()), Construct::Plain);
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

    fn fields_of<'a>(parent: Node<'a>, field: &str) -> Vec<Node<'a>> {
        let mut cursor = parent.walk();
        parent.children_by_field_name(field, &mut cursor).collect()
    }

    fn assert_heads_are_the_listed_fields(source: &str, parent_kind: &str, heads: &[&str]) {
        let tree = parse("a.rs", source);
        let parent = all_of_kind(tree.root_node(), parent_kind)[0];
        let head_ids: Vec<usize> = heads
            .iter()
            .flat_map(|field| {
                let nodes = fields_of(parent, field);
                assert!(!nodes.is_empty(), "{source}: no `{field}` child");
                nodes
            })
            .map(|node| node.id())
            .collect();
        for index in 0..parent.named_child_count() {
            let child = parent.named_child(index).expect("named child");
            assert_eq!(
                Rust.is_head(child, parent),
                head_ids.contains(&child.id()),
                "{source}: `{}` child of {parent_kind}",
                child.kind()
            );
        }
    }

    #[test]
    fn tuple_is_not_parenthesized() {
        let tree = parse("a.rs", "fn f(a: i32, b: i32) { let x = (a, b); }");
        let tuple = all_of_kind(tree.root_node(), "tuple_expression")[0];
        assert!(!Rust.is_parenthesized(tuple));
    }

    #[test]
    fn listed_fields_of_each_construct_are_heads_and_its_body_is_not() {
        assert_heads_are_the_listed_fields(
            "fn f(a: bool) { if a {} }",
            "if_expression",
            &["condition"],
        );
        assert_heads_are_the_listed_fields(
            "fn f(v: i32) { match v { _ => {} } }",
            "match_expression",
            &["value"],
        );
        assert_heads_are_the_listed_fields(
            "fn f(xs: Vec<i32>) { for x in xs {} }",
            "for_expression",
            &["pattern", "value"],
        );
        assert_heads_are_the_listed_fields(
            "fn f(a: bool) { while a {} }",
            "while_expression",
            &["condition"],
        );
    }

    #[test]
    fn conditional_branch_positions_are_not_overridden_for_rust() {
        let tree = parse("a.rs", "fn f(a: bool) { if a { } else { } }");
        let if_expression = all_of_kind(tree.root_node(), "if_expression")[0];
        let consequence = if_expression
            .child_by_field_name("consequence")
            .expect("consequence");
        assert!(!Rust.is_conditional_branch(consequence, if_expression));
    }
}
