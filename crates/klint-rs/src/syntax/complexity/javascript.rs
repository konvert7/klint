use super::grammar::{
    BooleanOperator, ComplexityGrammar, Construct, field_text, has_child_of_kind, operator_kind_is,
    text_of,
};
use tree_sitter::Node;

pub(super) struct JavaScript;

const FUNCTION_KINDS: &[&str] = &[
    "function_declaration",
    "function_expression",
    "arrow_function",
    "method_definition",
    "generator_function_declaration",
    "generator_function",
];

impl ComplexityGrammar for JavaScript {
    fn function_kinds(&self) -> &'static [&'static str] {
        FUNCTION_KINDS
    }

    fn construct(&self, node: Node<'_>, parent: Option<Node<'_>>) -> Construct {
        match node.kind() {
            "if_statement" if parent.is_some_and(|p| p.kind() == "else_clause") => {
                Construct::ChainedIf
            }
            "if_statement" | "ternary_expression" | "switch_statement" | "for_statement"
            | "for_in_statement" | "while_statement" | "do_statement" | "catch_clause" => {
                Construct::Nesting
            }
            "else_clause" if !has_child_of_kind(node, "if_statement") => Construct::Increment,
            "break_statement" | "continue_statement"
                if has_child_of_kind(node, "statement_identifier") =>
            {
                Construct::Increment
            }
            "finally_clause" => Construct::Increment,
            _ => Construct::Plain,
        }
    }

    fn is_boolean_expression(&self, node: Node<'_>) -> bool {
        node.kind() == "binary_expression" && operator_kind_is(node, &["&&", "||", "??"])
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
        field_text(function, "name", source)
    }

    fn assigned_name(&self, function: Node<'_>, source: &[u8]) -> Option<String> {
        let parent = function.parent()?;
        match parent.kind() {
            "variable_declarator" => identifier_field(parent, "name", source),
            "assignment_expression" => assigned_target_name(parent, source),
            "pair" => field_text(parent, "key", source).map(|key| trim_quotes(&key)),
            "public_field_definition" => field_text(parent, "name", source),
            _ => None,
        }
    }
}

fn identifier_field(node: Node<'_>, field: &str, source: &[u8]) -> Option<String> {
    let child = node.child_by_field_name(field)?;
    (child.kind() == "identifier").then(|| text_of(child, source))
}

fn assigned_target_name(assignment: Node<'_>, source: &[u8]) -> Option<String> {
    let left = assignment.child_by_field_name("left")?;
    match left.kind() {
        "identifier" => Some(text_of(left, source)),
        "member_expression" => field_text(left, "property", source),
        _ => None,
    }
}

fn trim_quotes(key: &str) -> String {
    key.trim_matches(['"', '\'', '`']).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::complexity::grammar::testing::{all_of_kind, parse};
    use tree_sitter::Tree;

    fn parsed(source: &str) -> Tree {
        parse("a.ts", source)
    }

    fn name_of(source: &str, kind: &str) -> Option<String> {
        let tree = parsed(source);
        let function = all_of_kind(tree.root_node(), kind)
            .into_iter()
            .next()
            .expect("function node");
        JavaScript.assigned_name(function, source.as_bytes())
    }

    #[test]
    fn if_in_else_is_chained_and_a_plain_else_increments() {
        let tree = parsed("if (a) {} else if (b) {} else {}");
        let ifs = all_of_kind(tree.root_node(), "if_statement");
        let else_clauses = all_of_kind(tree.root_node(), "else_clause");
        let inner = ifs[1];
        assert_eq!(
            JavaScript.construct(inner, inner.parent()),
            Construct::ChainedIf
        );
        assert_eq!(
            JavaScript.construct(else_clauses[1], else_clauses[1].parent()),
            Construct::Increment
        );
    }

    #[test]
    fn labeled_break_increments_and_plain_break_does_not() {
        let tree = parsed("outer: for (;;) { break outer; } for (;;) { break; }");
        let breaks = all_of_kind(tree.root_node(), "break_statement");
        assert_eq!(
            JavaScript.construct(breaks[0], breaks[0].parent()),
            Construct::Increment
        );
        assert_eq!(
            JavaScript.construct(breaks[1], breaks[1].parent()),
            Construct::Plain
        );
    }

    #[test]
    fn nullish_is_a_boolean_operator_but_logical_assignment_is_not() {
        let tree = parsed("a ?? b; a ||= b;");
        let binary = all_of_kind(tree.root_node(), "binary_expression");
        let assignment = all_of_kind(tree.root_node(), "augmented_assignment_expression");
        assert!(JavaScript.is_boolean_expression(binary[0]));
        assert!(!JavaScript.is_boolean_expression(assignment[0]));
    }

    #[test]
    fn variable_names_and_property_names_are_used_for_anonymous_functions() {
        assert_eq!(
            name_of("const inner = () => {};", "arrow_function"),
            Some("inner".to_string())
        );
        assert_eq!(
            name_of(
                "const o = { prop: function (a) {} };",
                "function_expression"
            ),
            Some("prop".to_string())
        );
        assert_eq!(name_of("xs.forEach((a) => {});", "arrow_function"), None);
    }
}
