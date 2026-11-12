use super::grammar::{
    BooleanOperator, ComplexityGrammar, Construct, field_text, is_field_of, is_in_any_field,
    operator_kind_is, text_of,
};
use tree_sitter::Node;

pub(super) struct CSharp;

const FUNCTION_KINDS: &[&str] = &[
    "method_declaration",
    "constructor_declaration",
    "destructor_declaration",
    "local_function_statement",
    "lambda_expression",
    "anonymous_method_expression",
    "accessor_declaration",
    "operator_declaration",
    "conversion_operator_declaration",
];

impl ComplexityGrammar for CSharp {
    fn scores_leaf_kind(&self, kind: &str) -> bool {
        kind == "else"
    }

    fn function_kinds(&self) -> &'static [&'static str] {
        FUNCTION_KINDS
    }

    fn construct(&self, node: Node<'_>, parent: Option<Node<'_>>) -> Construct {
        match node.kind() {
            "if_statement" if is_else_branch(node, parent) => Construct::ChainedIf,
            "if_statement"
            | "conditional_expression"
            | "switch_statement"
            | "switch_expression"
            | "for_statement"
            | "foreach_statement"
            | "while_statement"
            | "do_statement"
            | "catch_clause" => Construct::Nesting,
            "else" if is_else_of_if(parent) => Construct::Increment,
            "goto_statement" | "when_clause" => Construct::Increment,
            _ => Construct::Plain,
        }
    }

    fn is_boolean_expression(&self, node: Node<'_>) -> bool {
        (node.kind() == "binary_expression" && operator_kind_is(node, &["&&", "||", "??"]))
            || matches!(node.kind(), "and_pattern" | "or_pattern")
    }

    fn is_parenthesized(&self, node: Node<'_>) -> bool {
        matches!(
            node.kind(),
            "parenthesized_expression" | "parenthesized_pattern"
        )
    }

    fn is_head(&self, node: Node<'_>, parent: Node<'_>) -> bool {
        is_in_any_field(parent, head_fields(parent.kind()), node)
            || is_switch_expression_subject(parent, node)
            || is_catch_head(parent, node)
    }

    fn is_conditional_branch(&self, node: Node<'_>, parent: Node<'_>) -> bool {
        parent.kind() == "conditional_expression"
            && is_in_any_field(parent, &["consequence", "alternative"], node)
    }

    fn boolean_operator(&self, token_kind: &str) -> Option<BooleanOperator> {
        match token_kind {
            "&&" | "and" => Some(BooleanOperator::And),
            "||" | "or" => Some(BooleanOperator::Or),
            "??" => Some(BooleanOperator::Nullish),
            _ => None,
        }
    }

    fn declared_name(&self, function: Node<'_>, source: &[u8]) -> Option<String> {
        match function.kind() {
            "method_declaration"
            | "constructor_declaration"
            | "destructor_declaration"
            | "local_function_statement" => field_text(function, "name", source),
            "accessor_declaration" => accessor_owner_name(function, source),
            "operator_declaration" => {
                field_text(function, "operator", source).map(|op| format!("operator {op}"))
            }
            "conversion_operator_declaration" => {
                field_text(function, "type", source).map(|kind| format!("operator {kind}"))
            }
            _ => None,
        }
    }

    fn assigned_name(&self, function: Node<'_>, source: &[u8]) -> Option<String> {
        let parent = function.parent()?;
        match parent.kind() {
            "variable_declarator" => field_text(parent, "name", source),
            "assignment_expression" if is_field_of(parent, "right", function) => {
                assigned_target_name(parent, source)
            }
            _ => None,
        }
    }
}

fn is_else_branch(node: Node<'_>, parent: Option<Node<'_>>) -> bool {
    parent.is_some_and(|p| p.kind() == "if_statement" && is_field_of(p, "alternative", node))
}

fn head_fields(kind: &str) -> &'static [&'static str] {
    match kind {
        "if_statement" | "conditional_expression" | "while_statement" | "do_statement" => {
            &["condition"]
        }
        "switch_statement" => &["value"],
        "for_statement" => &["initializer", "condition", "update"],
        "foreach_statement" => &["left", "right", "type"],
        _ => &[],
    }
}

fn is_switch_expression_subject(parent: Node<'_>, node: Node<'_>) -> bool {
    parent.kind() == "switch_expression"
        && parent
            .named_child(0)
            .filter(|subject| subject.kind() != "switch_expression_arm")
            .is_some_and(|subject| subject.id() == node.id())
}

fn is_catch_head(parent: Node<'_>, node: Node<'_>) -> bool {
    parent.kind() == "catch_clause"
        && matches!(node.kind(), "catch_declaration" | "catch_filter_clause")
}

fn is_else_of_if(parent: Option<Node<'_>>) -> bool {
    parent.is_some_and(|p| {
        p.kind() == "if_statement"
            && p.child_by_field_name("alternative")
                .is_some_and(|alternative| alternative.kind() != "if_statement")
    })
}

fn accessor_owner_name(accessor: Node<'_>, source: &[u8]) -> Option<String> {
    let owner = accessor.parent()?.parent()?;
    match owner.kind() {
        "property_declaration" | "event_declaration" => field_text(owner, "name", source),
        "indexer_declaration" => Some("this".to_string()),
        _ => None,
    }
}

fn assigned_target_name(assignment: Node<'_>, source: &[u8]) -> Option<String> {
    let left = assignment.child_by_field_name("left")?;
    match left.kind() {
        "identifier" => Some(text_of(left, source)),
        "member_access_expression" => field_text(left, "name", source),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::complexity::grammar::testing::{all_of_kind, parse};
    use tree_sitter::Tree;

    #[test]
    fn else_if_is_chained_and_plain_else_increments() {
        let tree = parse(
            "a.cs",
            "class C { void M(bool a, bool b) { if (a) {} else if (b) {} else {} } }",
        );
        let ifs = all_of_kind(tree.root_node(), "if_statement");
        let elses = all_of_kind(tree.root_node(), "else");
        assert_eq!(
            CSharp.construct(ifs[1], ifs[1].parent()),
            Construct::ChainedIf
        );
        assert_eq!(
            CSharp.construct(elses[1], elses[1].parent()),
            Construct::Increment
        );
    }

    #[test]
    fn and_pattern_is_boolean_and_goto_increments() {
        let tree = parse(
            "a.cs",
            "class C { int M(int x) { switch (x) { case 1 or 2: goto case 3; } return 0; } }",
        );
        let patterns = all_of_kind(tree.root_node(), "or_pattern");
        let gotos = all_of_kind(tree.root_node(), "goto_statement");
        assert!(CSharp.is_boolean_expression(patterns[0]));
        assert_eq!(
            CSharp.construct(gotos[0], gotos[0].parent()),
            Construct::Increment
        );
    }

    #[test]
    fn lambda_assigned_to_a_delegate_takes_its_variable_name() {
        let source = "class C { void M() { Func<bool, int> g = x => 1; } }";
        let tree = parse("a.cs", source);
        let lambdas = all_of_kind(tree.root_node(), "lambda_expression");
        assert_eq!(
            CSharp.assigned_name(lambdas[0], source.as_bytes()),
            Some("g".to_string())
        );
    }

    fn parsed(source: &str) -> Tree {
        parse("a.cs", source)
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
                CSharp.is_head(child, parent),
                head_ids.contains(&child.id()),
                "{source}: `{}` child of {parent_kind}",
                child.kind()
            );
        }
    }

    #[test]
    fn finally_clause_adds_nothing() {
        let tree = parsed("class C { void M() { try { } finally { } } }");
        let finally = all_of_kind(tree.root_node(), "finally_clause")[0];
        assert_eq!(
            CSharp.construct(finally, finally.parent()),
            Construct::Plain
        );
    }

    #[test]
    fn parenthesized_or_pattern_is_parenthesized() {
        let tree = parsed("class C { bool M(int v) { return v is (2 or 3); } }");
        let pattern = all_of_kind(tree.root_node(), "parenthesized_pattern")[0];
        assert!(CSharp.is_parenthesized(pattern));
    }

    #[test]
    fn listed_fields_of_each_construct_are_heads_and_its_body_is_not() {
        assert_heads_are_the_listed_fields(
            "class C { void M(bool a) { if (a) { } } }",
            "if_statement",
            &["condition"],
        );
        assert_heads_are_the_listed_fields(
            "class C { void M(bool a) { var t = a ? 1 : 2; } }",
            "conditional_expression",
            &["condition"],
        );
        assert_heads_are_the_listed_fields(
            "class C { void M(int v) { switch (v) { default: break; } } }",
            "switch_statement",
            &["value"],
        );
        assert_heads_are_the_listed_fields(
            "class C { void M() { for (int i = 0; i < 3; i++) { } } }",
            "for_statement",
            &["initializer", "condition", "update"],
        );
        assert_heads_are_the_listed_fields(
            "class C { void M(int[] xs) { foreach (var x in xs) { } } }",
            "foreach_statement",
            &["left", "right", "type"],
        );
        assert_heads_are_the_listed_fields(
            "class C { void M(bool a) { while (a) { } } }",
            "while_statement",
            &["condition"],
        );
        assert_heads_are_the_listed_fields(
            "class C { void M(bool a) { do { } while (a); } }",
            "do_statement",
            &["condition"],
        );
    }

    #[test]
    fn switch_expression_subject_is_its_first_named_child_and_its_arms_are_not() {
        let tree = parsed(
            "class C { void M(bool a, bool b, bool c) { var s = (a ? b : c) switch { true => 1, _ => 2 }; } }",
        );
        let switch = all_of_kind(tree.root_node(), "switch_expression")[0];
        assert!(CSharp.is_head(switch.named_child(0).expect("subject"), switch));
        assert!(!CSharp.is_head(switch.named_child(1).expect("arm"), switch));
    }

    #[test]
    fn catch_declaration_and_filter_are_heads_and_the_catch_body_is_not() {
        let tree = parsed(
            "class C { void M(bool a) { try { } catch (System.Exception e) when (a) { } } }",
        );
        let catch = all_of_kind(tree.root_node(), "catch_clause")[0];
        for index in 0..catch.named_child_count() {
            let child = catch.named_child(index).expect("catch child");
            let expected = matches!(child.kind(), "catch_declaration" | "catch_filter_clause");
            assert_eq!(CSharp.is_head(child, catch), expected, "{}", child.kind());
        }
    }

    #[test]
    fn conditional_branches_are_conditional_branches_and_its_test_is_not() {
        let tree = parsed("class C { void M(bool a) { var t = a ? 1 : 2; } }");
        let conditional = all_of_kind(tree.root_node(), "conditional_expression")[0];
        let branch = |field: &str| conditional.child_by_field_name(field).expect("field");
        assert!(CSharp.is_conditional_branch(branch("consequence"), conditional));
        assert!(CSharp.is_conditional_branch(branch("alternative"), conditional));
        assert!(!CSharp.is_conditional_branch(branch("condition"), conditional));
    }
}
