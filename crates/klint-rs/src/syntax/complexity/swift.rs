use super::grammar::{
    BooleanOperator, ComplexityGrammar, Construct, field_text, first_descendant_of_kind, text_of,
};
use tree_sitter::Node;

pub(super) struct Swift;

const FUNCTION_KINDS: &[&str] = &[
    "function_declaration",
    "init_declaration",
    "deinit_declaration",
    "lambda_literal",
    "computed_getter",
    "computed_setter",
];

const BOOLEAN_KINDS: &[&str] = &[
    "conjunction_expression",
    "disjunction_expression",
    "nil_coalescing_expression",
];

impl ComplexityGrammar for Swift {
    fn function_kinds(&self) -> &'static [&'static str] {
        FUNCTION_KINDS
    }

    fn construct(&self, node: Node<'_>, parent: Option<Node<'_>>) -> Construct {
        match node.kind() {
            "if_statement" if parent.is_some_and(|p| p.kind() == "if_statement") => {
                Construct::ChainedIf
            }
            "if_statement"
            | "ternary_expression"
            | "switch_statement"
            | "for_statement"
            | "while_statement"
            | "repeat_while_statement"
            | "catch_block"
            | "guard_statement" => Construct::Nesting,
            "else" if is_if_else(node, parent) => Construct::Increment,
            "control_transfer_statement" if is_labeled_jump(node) => Construct::Increment,
            "where_keyword" if parent.is_some_and(|p| p.kind() == "switch_entry") => {
                Construct::Increment
            }
            _ => Construct::Plain,
        }
    }

    fn is_boolean_expression(&self, node: Node<'_>) -> bool {
        BOOLEAN_KINDS.contains(&node.kind())
    }

    fn boolean_operator(&self, token_kind: &str) -> Option<BooleanOperator> {
        match token_kind {
            "&&" => Some(BooleanOperator::And),
            "||" => Some(BooleanOperator::Or),
            "??" => Some(BooleanOperator::Nullish),
            _ => None,
        }
    }

    fn declared_name(&self, function: Node<'_>, source: &[u8]) -> Option<String> {
        match function.kind() {
            "function_declaration" => field_text(function, "name", source),
            "init_declaration" => Some("init".to_string()),
            "deinit_declaration" => Some("deinit".to_string()),
            "computed_getter" | "computed_setter" => enclosing_property_name(function, source),
            _ => None,
        }
    }

    fn assigned_name(&self, function: Node<'_>, source: &[u8]) -> Option<String> {
        let parent = function.parent()?;
        match parent.kind() {
            "property_declaration" => property_name(parent, source),
            _ => None,
        }
    }
}

fn is_if_else(node: Node<'_>, parent: Option<Node<'_>>) -> bool {
    parent.is_some_and(|p| p.kind() == "if_statement")
        && node
            .next_sibling()
            .is_none_or(|next| next.kind() != "if_statement")
}

fn is_labeled_jump(node: Node<'_>) -> bool {
    node.child(0)
        .is_some_and(|keyword| matches!(keyword.kind(), "break" | "continue"))
        && node.child_by_field_name("result").is_some()
}

fn enclosing_property_name(accessor: Node<'_>, source: &[u8]) -> Option<String> {
    let mut current = accessor.parent();
    while let Some(ancestor) = current {
        match ancestor.kind() {
            "property_declaration" => return property_name(ancestor, source),
            "class_body" | "protocol_body" | "source_file" => return None,
            _ => current = ancestor.parent(),
        }
    }
    None
}

fn property_name(property: Node<'_>, source: &[u8]) -> Option<String> {
    let name = property.child_by_field_name("name")?;
    first_descendant_of_kind(name, "simple_identifier")
        .map(|identifier| text_of(identifier, source))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::complexity::grammar::testing::{all_of_kind, parse};

    #[test]
    fn else_if_is_chained_and_guard_else_is_plain() {
        let tree = parse(
            "a.swift",
            "func f(a: Bool) {\n    if a {} else if a {} else {}\n    guard a else { return }\n}\n",
        );
        let ifs = all_of_kind(tree.root_node(), "if_statement");
        let elses = all_of_kind(tree.root_node(), "else");
        let guard_else = elses[2];
        assert_eq!(
            Swift.construct(ifs[1], ifs[1].parent()),
            Construct::ChainedIf
        );
        assert_eq!(
            Swift.construct(elses[0], elses[0].parent()),
            Construct::Plain
        );
        assert_eq!(
            Swift.construct(guard_else, guard_else.parent()),
            Construct::Plain
        );
    }

    #[test]
    fn labeled_continue_increments_and_bare_continue_does_not() {
        let tree = parse(
            "a.swift",
            "func f() {\n    outer: for x in xs { continue outer }\n    for x in xs { continue }\n}\n",
        );
        let jumps = all_of_kind(tree.root_node(), "control_transfer_statement");
        assert_eq!(
            Swift.construct(jumps[0], jumps[0].parent()),
            Construct::Increment
        );
        assert_eq!(
            Swift.construct(jumps[1], jumps[1].parent()),
            Construct::Plain
        );
    }

    #[test]
    fn conjunction_and_disjunction_are_boolean_expressions() {
        let tree = parse("a.swift", "let v = a && b || c\n");
        let conjunctions = all_of_kind(tree.root_node(), "conjunction_expression");
        assert!(Swift.is_boolean_expression(conjunctions[0]));
    }

    #[test]
    fn property_takes_the_name_of_a_closure_it_is_assigned() {
        let source = "let g = { x in x }\nxs.forEach { x in x }\n";
        let tree = parse("a.swift", source);
        let closures = all_of_kind(tree.root_node(), "lambda_literal");
        assert_eq!(
            Swift.assigned_name(closures[0], source.as_bytes()),
            Some("g".to_string())
        );
        assert_eq!(Swift.assigned_name(closures[1], source.as_bytes()), None);
    }
}
