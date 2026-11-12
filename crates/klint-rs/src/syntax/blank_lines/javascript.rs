use super::grammar::{
    Grammar, Role, Target, body_target, ends_with_braced_body, field_nodes, named_children,
    node_text,
};
use super::keyword::Keyword;
use tree_sitter::Node;

pub(super) struct JavaScript;

const CONTAINERS: &[&str] = &[
    "program",
    "statement_block",
    "switch_case",
    "switch_default",
];

impl Grammar for JavaScript {
    fn containers(&self) -> &'static [&'static str] {
        CONTAINERS
    }

    fn is_body_member(&self, child: Node<'_>, field: Option<&str>) -> bool {
        child.is_named() && field != Some("value")
    }

    fn classify<'a>(&self, member: Node<'a>) -> Role<'a> {
        if matches!(member.kind(), "comment" | "hash_bang_line") {
            return Role::Comment;
        }

        Role::Statement(target_of(unwrap_labels(member)))
    }

    fn assigned_names(&self, statement: Node<'_>, source: &[u8]) -> Vec<String> {
        let node = unwrap_labels(statement);
        match node.kind() {
            "lexical_declaration" | "variable_declaration" => declared_names(node, source),
            "expression_statement" => named_children(node)
                .into_iter()
                .next()
                .map_or_else(Vec::new, |expression| {
                    assigned_identifier(expression, source)
                }),
            _ => Vec::new(),
        }
    }

    fn is_condition_name(&self, node: Node<'_>) -> bool {
        matches!(node.kind(), "identifier" | "shorthand_property_identifier")
    }
}

fn unwrap_labels(node: Node<'_>) -> Node<'_> {
    match node.kind() {
        "labeled_statement" => node.child_by_field_name("body").map_or(node, unwrap_labels),
        _ => node,
    }
}

fn target_of(node: Node<'_>) -> Option<Target<'_>> {
    let target = match node.kind() {
        "if_statement" => Target {
            keyword: Keyword::If,
            written: "if",
            ends_with_block: ends_with_braced_body(node, "statement_block", Some("else_clause")),
            conditions: field_nodes(node, "condition"),
        },
        "for_statement" | "for_in_statement" => {
            body_target(node, Keyword::For, "for", "statement_block")
        }
        "while_statement" => body_target(node, Keyword::While, "while", "statement_block"),
        "do_statement" => Target {
            keyword: Keyword::While,
            written: "do",
            ends_with_block: false,
            conditions: Vec::new(),
        },
        "switch_statement" => Target {
            keyword: Keyword::Switch,
            written: "switch",
            ends_with_block: true,
            conditions: field_nodes(node, "value"),
        },
        "try_statement" => Target {
            keyword: Keyword::Try,
            written: "try",
            ends_with_block: true,
            conditions: Vec::new(),
        },
        "return_statement" => Target {
            keyword: Keyword::Return,
            written: "return",
            ends_with_block: false,
            conditions: Vec::new(),
        },
        _ => return None,
    };
    Some(target)
}

fn declared_names(declaration: Node<'_>, source: &[u8]) -> Vec<String> {
    named_children(declaration)
        .into_iter()
        .filter(|child| child.kind() == "variable_declarator")
        .filter_map(|declarator| declarator.child_by_field_name("name"))
        .flat_map(|name| pattern_names(name, source))
        .collect()
}

fn assigned_identifier(expression: Node<'_>, source: &[u8]) -> Vec<String> {
    match expression.kind() {
        "assignment_expression" | "augmented_assignment_expression" => expression
            .child_by_field_name("left")
            .filter(|left| left.kind() == "identifier")
            .map(|left| vec![node_text(left, source)])
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn pattern_names(pattern: Node<'_>, source: &[u8]) -> Vec<String> {
    match pattern.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => vec![node_text(pattern, source)],
        "object_pattern" | "array_pattern" | "rest_pattern" => named_children(pattern)
            .into_iter()
            .flat_map(|child| pattern_names(child, source))
            .collect(),
        "pair_pattern" => field_pattern_names(pattern, "value", source),
        "assignment_pattern" | "object_assignment_pattern" => {
            field_pattern_names(pattern, "left", source)
        }
        _ => Vec::new(),
    }
}

fn field_pattern_names(pattern: Node<'_>, field: &str, source: &[u8]) -> Vec<String> {
    pattern
        .child_by_field_name(field)
        .map_or_else(Vec::new, |child| pattern_names(child, source))
}

#[cfg(test)]
mod tests {
    use super::super::grammar::descendants_of_kind;
    use super::*;
    use crate::syntax::parse_source;
    use std::path::Path;

    fn assigned_names_of(source: &str, statement_kind: &str) -> Vec<String> {
        let tree = parse_source(Path::new("a.ts"), source).expect("source parses");
        let statement = descendants_of_kind(tree.root_node(), &[statement_kind])
            .into_iter()
            .next()
            .expect("statement of the given kind");
        JavaScript.assigned_names(statement, source.as_bytes())
    }

    #[test]
    fn plain_assignment_names_its_target() {
        assert_eq!(
            assigned_names_of("x = f();", "expression_statement"),
            vec!["x".to_string()]
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
    fn object_destructuring_names_every_binding() {
        assert_eq!(
            assigned_names_of("const { a, b } = f();", "lexical_declaration"),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn array_destructuring_with_rest_names_every_binding() {
        assert_eq!(
            assigned_names_of("const [p, ...q] = f();", "lexical_declaration"),
            vec!["p".to_string(), "q".to_string()]
        );
    }

    #[test]
    fn declaration_with_two_declarators_names_both() {
        assert_eq!(
            assigned_names_of("let a = 1, b = 2;", "lexical_declaration"),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn member_assignment_names_nothing() {
        assert!(assigned_names_of("self.x = f();", "expression_statement").is_empty());
    }

    #[test]
    fn element_assignment_names_nothing() {
        assert!(assigned_names_of("a[0] = 1;", "expression_statement").is_empty());
    }

    #[test]
    fn call_statement_names_nothing() {
        assert!(assigned_names_of("foo();", "expression_statement").is_empty());
    }
}
