use super::grammar::{
    BooleanOperator, ComplexityGrammar, Construct, field_text, has_child_of_kind, is_in_any_field,
    operator_kind_is, text_of,
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
            _ => Construct::Plain,
        }
    }

    fn is_boolean_expression(&self, node: Node<'_>) -> bool {
        node.kind() == "binary_expression" && operator_kind_is(node, &["&&", "||", "??"])
    }

    fn skips_sequence(
        &self,
        root: Node<'_>,
        enclosing: Option<Node<'_>>,
        operators: &[BooleanOperator],
    ) -> bool {
        enclosing.is_some_and(|parent| parent.kind() == "jsx_expression")
            && operators.windows(2).all(|pair| pair[0] == pair[1])
            && !has_conditional_operand(root)
    }

    fn is_head(&self, node: Node<'_>, parent: Node<'_>) -> bool {
        is_in_any_field(parent, head_fields(parent.kind()), node)
    }

    fn is_conditional_branch(&self, node: Node<'_>, parent: Node<'_>) -> bool {
        parent.kind() == "ternary_expression"
            && is_in_any_field(parent, &["consequence", "alternative"], node)
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

fn head_fields(kind: &str) -> &'static [&'static str] {
    match kind {
        "if_statement" | "ternary_expression" | "while_statement" | "do_statement" => {
            &["condition"]
        }
        "switch_statement" => &["value"],
        "for_statement" => &["initializer", "condition", "increment"],
        "for_in_statement" => &["left", "right", "value"],
        "catch_clause" => &["parameter", "type"],
        _ => &[],
    }
}

fn has_conditional_operand(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|child| {
        if is_sequence_member(child) {
            has_conditional_operand(child)
        } else {
            child.is_named() && child.kind() == "ternary_expression"
        }
    })
}

fn is_sequence_member(node: Node<'_>) -> bool {
    JavaScript.is_boolean_expression(node) || node.kind() == "parenthesized_expression"
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

    fn parsed_tsx(source: &str) -> Tree {
        parse("a.tsx", source)
    }

    fn skips(source: &str, operators: &[BooleanOperator]) -> bool {
        let tree = parsed_tsx(source);
        let root = all_of_kind(tree.root_node(), "binary_expression")[0];
        JavaScript.skips_sequence(root, root.parent(), operators)
    }

    fn fields_of<'a>(parent: Node<'a>, field: &str) -> Vec<Node<'a>> {
        let mut cursor = parent.walk();
        parent.children_by_field_name(field, &mut cursor).collect()
    }

    fn assert_heads_are_the_listed_fields(source: &str, parent_kind: &str, heads: &[&str]) {
        let tree = parsed(source);
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
                JavaScript.is_head(child, parent),
                head_ids.contains(&child.id()),
                "{source}: `{}` child of {parent_kind}",
                child.kind()
            );
        }
    }

    #[test]
    fn finally_clause_adds_nothing() {
        let tree = parsed("try {} catch (e) {} finally {}");
        let finally = all_of_kind(tree.root_node(), "finally_clause")[0];
        assert_eq!(
            JavaScript.construct(finally, finally.parent()),
            Construct::Plain
        );
    }

    #[test]
    fn catch_clause_still_nests() {
        let tree = parsed("try {} catch (e) {} finally {}");
        let catch = all_of_kind(tree.root_node(), "catch_clause")[0];
        assert_eq!(
            JavaScript.construct(catch, catch.parent()),
            Construct::Nesting
        );
    }

    #[test]
    fn parenthesized_or_expression_is_parenthesized() {
        let tree = parsed("x = (b || c);");
        let parens = all_of_kind(tree.root_node(), "parenthesized_expression")[0];
        assert!(JavaScript.is_parenthesized(parens));
    }

    #[test]
    fn jsx_single_and_directly_in_braces_skips_the_sequence() {
        assert!(skips("<div>{a && <X/>}</div>;", &[BooleanOperator::And]));
    }

    #[test]
    fn jsx_repeated_and_directly_in_braces_skips_the_sequence() {
        assert!(skips(
            "<div>{a && b && <X/>}</div>;",
            &[BooleanOperator::And, BooleanOperator::And]
        ));
    }

    #[test]
    fn jsx_mixed_and_or_is_counted() {
        assert!(!skips(
            "<div>{a && b || <X/>}</div>;",
            &[BooleanOperator::And, BooleanOperator::Or]
        ));
    }

    #[test]
    fn jsx_and_with_a_ternary_operand_is_counted() {
        assert!(!skips(
            "<div>{a && (b ? <X/> : <Y/>)}</div>;",
            &[BooleanOperator::And]
        ));
    }

    #[test]
    fn and_returned_outside_jsx_is_counted() {
        assert!(!skips(
            "function f(a, b) { return a && b; }",
            &[BooleanOperator::And]
        ));
    }

    #[test]
    fn listed_fields_of_each_construct_are_heads_and_its_body_is_not() {
        assert_heads_are_the_listed_fields("if (a) {}", "if_statement", &["condition"]);
        assert_heads_are_the_listed_fields("x = a ? b : c;", "ternary_expression", &["condition"]);
        assert_heads_are_the_listed_fields(
            "switch (v) { default: break; }",
            "switch_statement",
            &["value"],
        );
        assert_heads_are_the_listed_fields(
            "for (let i = 0; i < n; i++) {}",
            "for_statement",
            &["initializer", "condition", "increment"],
        );
        assert_heads_are_the_listed_fields(
            "for (var x = 1 in y) {}",
            "for_in_statement",
            &["left", "right", "value"],
        );
        assert_heads_are_the_listed_fields("while (a) {}", "while_statement", &["condition"]);
        assert_heads_are_the_listed_fields("do {} while (a);", "do_statement", &["condition"]);
        assert_heads_are_the_listed_fields(
            "try {} catch (e: Error) {}",
            "catch_clause",
            &["parameter", "type"],
        );
    }

    #[test]
    fn ternary_branches_are_conditional_branches_and_its_test_is_not() {
        let tree = parsed("a ? b : c;");
        let ternary = all_of_kind(tree.root_node(), "ternary_expression")[0];
        let branch = |field: &str| ternary.child_by_field_name(field).expect("ternary field");
        assert!(JavaScript.is_conditional_branch(branch("consequence"), ternary));
        assert!(JavaScript.is_conditional_branch(branch("alternative"), ternary));
        assert!(!JavaScript.is_conditional_branch(branch("condition"), ternary));
    }
}
