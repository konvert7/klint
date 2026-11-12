use super::grammar::{
    BooleanOperator, ComplexityGrammar, Construct, field_text, is_field_of, is_in_any_field,
    text_of,
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
            _ => Construct::Plain,
        }
    }

    fn is_boolean_expression(&self, node: Node<'_>) -> bool {
        node.kind() == "boolean_operator"
    }

    fn is_head(&self, node: Node<'_>, parent: Node<'_>) -> bool {
        match parent.kind() {
            "conditional_expression" => is_conditional_test(parent, node),
            "except_group_clause" => node.is_named() && node.kind() != "block",
            kind => is_in_any_field(parent, head_fields(kind), node),
        }
    }

    fn is_conditional_branch(&self, node: Node<'_>, parent: Node<'_>) -> bool {
        parent.kind() == "conditional_expression"
            && [0, 2].iter().any(|&index| {
                parent
                    .named_child(index)
                    .is_some_and(|branch| branch.id() == node.id())
            })
    }

    fn boolean_operator(&self, token_kind: &str) -> Option<BooleanOperator> {
        match token_kind {
            "and" => Some(BooleanOperator::And),
            "or" => Some(BooleanOperator::Or),
            _ => None,
        }
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

fn is_conditional_test(conditional: Node<'_>, node: Node<'_>) -> bool {
    conditional
        .named_child(1)
        .is_some_and(|test| test.id() == node.id())
}

fn head_fields(kind: &str) -> &'static [&'static str] {
    match kind {
        "if_statement" | "elif_clause" | "while_statement" => &["condition"],
        "match_statement" => &["subject"],
        "for_statement" => &["left", "right"],
        "except_clause" => &["value", "alias"],
        _ => &[],
    }
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

    fn fields_of<'a>(parent: Node<'a>, field: &str) -> Vec<Node<'a>> {
        let mut cursor = parent.walk();
        parent.children_by_field_name(field, &mut cursor).collect()
    }

    fn assert_heads_are_the_listed_fields(source: &str, parent_kind: &str, heads: &[&str]) {
        let tree = parse("a.py", source);
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
                Python.is_head(child, parent),
                head_ids.contains(&child.id()),
                "{source}: `{}` child of {parent_kind}",
                child.kind()
            );
        }
    }

    #[test]
    fn finally_clause_adds_nothing() {
        let tree = parse("a.py", "try:\n    pass\nfinally:\n    pass\n");
        let finally = all_of_kind(tree.root_node(), "finally_clause")[0];
        assert_eq!(
            Python.construct(finally, finally.parent()),
            Construct::Plain
        );
    }

    #[test]
    fn else_of_a_try_still_increments() {
        let tree = parse(
            "a.py",
            "try:\n    pass\nexcept Exception:\n    pass\nelse:\n    pass\n",
        );
        let else_clause = all_of_kind(tree.root_node(), "else_clause")[0];
        assert_eq!(
            Python.construct(else_clause, else_clause.parent()),
            Construct::Increment
        );
    }

    #[test]
    fn parenthesized_tuple_is_not_parenthesized() {
        let tree = parse("a.py", "x = (a, b)\n");
        let tuple = all_of_kind(tree.root_node(), "tuple")[0];
        assert!(!Python.is_parenthesized(tuple));
    }

    #[test]
    fn listed_fields_of_each_construct_are_heads_and_its_body_is_not() {
        assert_heads_are_the_listed_fields("if a:\n    pass\n", "if_statement", &["condition"]);
        assert_heads_are_the_listed_fields(
            "if a:\n    pass\nelif b:\n    pass\n",
            "elif_clause",
            &["condition"],
        );
        assert_heads_are_the_listed_fields(
            "match v:\n    case 1:\n        pass\n",
            "match_statement",
            &["subject"],
        );
        assert_heads_are_the_listed_fields(
            "for x in xs:\n    pass\n",
            "for_statement",
            &["left", "right"],
        );
        assert_heads_are_the_listed_fields(
            "while c:\n    pass\n",
            "while_statement",
            &["condition"],
        );
        assert_heads_are_the_listed_fields(
            "try:\n    pass\nexcept Exception as e:\n    pass\n",
            "except_clause",
            &["value"],
        );
    }

    #[test]
    fn every_named_child_of_an_except_star_clause_but_the_block_is_a_head() {
        let tree = parse("a.py", "try:\n    pass\nexcept* ValueError:\n    pass\n");
        let group = all_of_kind(tree.root_node(), "except_group_clause")[0];
        assert!(Python.is_head(group.named_child(0).expect("exception"), group));
        assert!(!Python.is_head(group.named_child(1).expect("block"), group));
    }

    #[test]
    fn only_the_middle_operand_of_a_conditional_expression_is_its_head() {
        let tree = parse("a.py", "1 if a else 2\n");
        let conditional = all_of_kind(tree.root_node(), "conditional_expression")[0];
        let operand = |index: usize| conditional.named_child(index).expect("operand");
        assert!(!Python.is_head(operand(0), conditional));
        assert!(Python.is_head(operand(1), conditional));
        assert!(!Python.is_head(operand(2), conditional));
    }

    #[test]
    fn conditional_branches_are_conditional_branches_and_its_test_is_not() {
        let tree = parse("a.py", "1 if a else 2\n");
        let conditional = all_of_kind(tree.root_node(), "conditional_expression")[0];
        let operand = |index: usize| conditional.named_child(index).expect("operand");
        assert!(Python.is_conditional_branch(operand(0), conditional));
        assert!(Python.is_conditional_branch(operand(2), conditional));
        assert!(!Python.is_conditional_branch(operand(1), conditional));
    }
}
