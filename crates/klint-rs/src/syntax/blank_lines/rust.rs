use super::grammar::{
    Grammar, Role, Target, block_target, field_nodes, is_field_of, named_children, node_text,
};
use super::keyword::Keyword;
use tree_sitter::Node;

pub(super) struct Rust;

const CONTAINERS: &[&str] = &["source_file", "block"];

impl Grammar for Rust {
    fn containers(&self) -> &'static [&'static str] {
        CONTAINERS
    }

    fn is_body_member(&self, child: Node<'_>, _field: Option<&str>) -> bool {
        child.is_named() && child.kind() != "label"
    }

    fn classify<'a>(&self, member: Node<'a>) -> Role<'a> {
        match member.kind() {
            "line_comment" | "block_comment" => Role::Comment,
            "attribute_item" => Role::Prefix,
            _ => Role::Statement(target_of(statement_expression(member))),
        }
    }

    fn assigned_names(&self, statement: Node<'_>, source: &[u8]) -> Vec<String> {
        match statement.kind() {
            "expression_statement" => assigned_expression(statement_expression(statement), source),
            "let_declaration" => statement
                .child_by_field_name("pattern")
                .map_or_else(Vec::new, |pattern| pattern_names(pattern, source)),
            _ => Vec::new(),
        }
    }

    fn is_condition_name(&self, node: Node<'_>) -> bool {
        node.kind() == "identifier"
            && node
                .parent()
                .is_none_or(|parent| parent.kind() != "scoped_identifier")
    }
}

fn statement_expression(member: Node<'_>) -> Node<'_> {
    match member.kind() {
        "expression_statement" => named_children(member).into_iter().next().unwrap_or(member),
        _ => member,
    }
}

fn target_of(node: Node<'_>) -> Option<Target<'_>> {
    let target = match node.kind() {
        "if_expression" => Target {
            keyword: Keyword::If,
            written: "if",
            ends_with_block: true,
            conditions: field_nodes(node, "condition"),
        },
        "match_expression" => Target {
            keyword: Keyword::Switch,
            written: "match",
            ends_with_block: true,
            conditions: field_nodes(node, "value"),
        },
        "loop_expression" => block_target(Keyword::Loop, "loop"),
        "while_expression" => block_target(Keyword::While, "while"),
        "for_expression" => block_target(Keyword::For, "for"),
        "return_expression" => Target {
            keyword: Keyword::Return,
            written: "return",
            ends_with_block: false,
            conditions: Vec::new(),
        },
        _ => return None,
    };
    Some(target)
}

fn assigned_expression(expression: Node<'_>, source: &[u8]) -> Vec<String> {
    match expression.kind() {
        "assignment_expression" | "compound_assignment_expr" => expression
            .child_by_field_name("left")
            .filter(|left| left.kind() == "identifier")
            .map(|left| vec![node_text(left, source)])
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn pattern_names(pattern: Node<'_>, source: &[u8]) -> Vec<String> {
    match pattern.kind() {
        "identifier" => vec![node_text(pattern, source)],
        "tuple_struct_pattern" | "struct_pattern" => named_children(pattern)
            .into_iter()
            .filter(|child| !is_field_of(pattern, "type", *child))
            .flat_map(|child| pattern_names(child, source))
            .collect(),
        "field_pattern" => field_pattern_names(pattern, source),
        "tuple_pattern" | "slice_pattern" | "ref_pattern" | "mut_pattern" | "reference_pattern"
        | "or_pattern" | "captured_pattern" => named_children(pattern)
            .into_iter()
            .flat_map(|child| pattern_names(child, source))
            .collect(),
        _ => Vec::new(),
    }
}

fn field_pattern_names(pattern: Node<'_>, source: &[u8]) -> Vec<String> {
    match pattern.child_by_field_name("pattern") {
        Some(inner) => pattern_names(inner, source),
        None => pattern
            .child_by_field_name("name")
            .map_or_else(Vec::new, |name| vec![node_text(name, source)]),
    }
}

#[cfg(test)]
mod tests {
    use super::super::grammar::descendants_of_kind;
    use super::*;
    use crate::syntax::parse_source;
    use std::path::Path;

    fn assigned_names_of(statement: &str, statement_kind: &str) -> Vec<String> {
        let source = format!("fn main() {{\n{statement}\n}}\n");
        let tree = parse_source(Path::new("a.rs"), &source).expect("source parses");
        let node = descendants_of_kind(tree.root_node(), &[statement_kind])
            .into_iter()
            .next()
            .expect("statement of the given kind");
        Rust.assigned_names(node, source.as_bytes())
    }

    #[test]
    fn let_binding_names_its_target() {
        assert_eq!(
            assigned_names_of("let x = f();", "let_declaration"),
            vec!["x".to_string()]
        );
    }

    #[test]
    fn let_else_binding_names_its_pattern() {
        assert_eq!(
            assigned_names_of("let Some(q) = f() else { return; };", "let_declaration"),
            vec!["q".to_string()]
        );
    }

    #[test]
    fn tuple_binding_names_every_element_including_mut() {
        assert_eq!(
            assigned_names_of("let (a, mut b) = f();", "let_declaration"),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn struct_binding_names_shorthand_and_renamed_fields() {
        assert_eq!(
            assigned_names_of("let Point { x, y: z } = p;", "let_declaration"),
            vec!["x".to_string(), "z".to_string()]
        );
    }

    #[test]
    fn compound_assignment_names_its_target() {
        assert_eq!(
            assigned_names_of("x += 1;", "expression_statement"),
            vec!["x".to_string()]
        );
    }

    #[test]
    fn field_assignment_names_nothing() {
        assert!(assigned_names_of("self.x = f();", "expression_statement").is_empty());
    }
}
