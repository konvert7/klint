use super::grammar::{
    BooleanOperator, ComplexityGrammar, Construct, field_text, is_field_of, operator_kind_is,
    text_of,
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
            "finally_clause" => Construct::Increment,
            _ => Construct::Plain,
        }
    }

    fn is_boolean_expression(&self, node: Node<'_>) -> bool {
        (node.kind() == "binary_expression" && operator_kind_is(node, &["&&", "||", "??"]))
            || matches!(node.kind(), "and_pattern" | "or_pattern")
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
}
