use super::grammar::{
    BooleanOperator, ComplexityGrammar, Construct, field_text, is_field_of, text_of,
};
use tree_sitter::Node;

pub(super) struct Python;

const FUNCTION_KINDS: &[&str] = &["function_definition", "lambda"];

const COMPREHENSION_KINDS: &[&str] = &[
    "list_comprehension",
    "set_comprehension",
    "dictionary_comprehension",
    "generator_expression",
];

impl ComplexityGrammar for Python {
    fn function_kinds(&self) -> &'static [&'static str] {
        FUNCTION_KINDS
    }

    fn construct(&self, node: Node<'_>, parent: Option<Node<'_>>) -> Construct {
        match node.kind() {
            "if_statement"
            | "conditional_expression"
            | "match_statement"
            | "for_statement"
            | "while_statement"
            | "except_clause"
            | "except_group_clause" => Construct::Nesting,
            "elif_clause" => Construct::ChainedIf,
            "else_clause" if is_loop_else(parent) => Construct::LoopElse,
            "else_clause" => Construct::Increment,
            "if_clause" if is_comprehension(parent) => Construct::Increment,
            "finally_clause" => Construct::NestingOnly,
            _ => Construct::Plain,
        }
    }

    fn is_boolean_expression(&self, node: Node<'_>) -> bool {
        node.kind() == "boolean_operator"
    }

    fn boolean_operator(&self, token_kind: &str) -> Option<BooleanOperator> {
        match token_kind {
            "and" => Some(BooleanOperator::And),
            "or" => Some(BooleanOperator::Or),
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
        if parent.kind() != "assignment" || !is_field_of(parent, "right", function) {
            return None;
        }
        let left = parent.child_by_field_name("left")?;
        (left.kind() == "identifier").then(|| text_of(left, source))
    }
}

fn is_loop_else(parent: Option<Node<'_>>) -> bool {
    parent.is_some_and(|p| matches!(p.kind(), "for_statement" | "while_statement"))
}

fn is_comprehension(parent: Option<Node<'_>>) -> bool {
    parent.is_some_and(|p| COMPREHENSION_KINDS.contains(&p.kind()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::complexity::grammar::testing::{all_of_kind, parse};

    #[test]
    fn elif_chains_and_loop_else_is_its_own_construct() {
        let tree = parse(
            "a.py",
            "def f(xs, a):\n    for x in xs:\n        pass\n    else:\n        pass\n    if a:\n        pass\n    elif a:\n        pass\n",
        );
        let else_clauses = all_of_kind(tree.root_node(), "else_clause");
        let elifs = all_of_kind(tree.root_node(), "elif_clause");
        assert_eq!(
            Python.construct(else_clauses[0], else_clauses[0].parent()),
            Construct::LoopElse
        );
        assert_eq!(
            Python.construct(elifs[0], elifs[0].parent()),
            Construct::ChainedIf
        );
    }

    #[test]
    fn comprehension_if_increments_and_match_guard_does_not() {
        let tree = parse(
            "a.py",
            "xs = [x for x in y if x]\nmatch v:\n    case 1 if b:\n        pass\n",
        );
        let if_clauses = all_of_kind(tree.root_node(), "if_clause");
        assert_eq!(
            Python.construct(if_clauses[0], if_clauses[0].parent()),
            Construct::Increment
        );
        assert_eq!(
            Python.construct(if_clauses[1], if_clauses[1].parent()),
            Construct::Plain
        );
    }

    #[test]
    fn lambda_takes_the_name_of_the_variable_it_is_assigned_to() {
        let tree = parse("a.py", "fn = lambda x: 1\nsorted(xs, key=lambda x: x)\n");
        let lambdas = all_of_kind(tree.root_node(), "lambda");
        assert_eq!(
            Python.assigned_name(
                lambdas[0],
                b"fn = lambda x: 1\nsorted(xs, key=lambda x: x)\n"
            ),
            Some("fn".to_string())
        );
        assert_eq!(
            Python.assigned_name(
                lambdas[1],
                b"fn = lambda x: 1\nsorted(xs, key=lambda x: x)\n"
            ),
            None
        );
    }
}
