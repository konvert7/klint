use super::grammar::{
    Grammar, Role, Target, block_target, body_target, descendants_of_kind, ends_with_braced_body,
    field_nodes, is_field_of, named_children, node_text,
};
use super::keyword::Keyword;
use tree_sitter::Node;

pub(super) struct CSharp;

const CONTAINERS: &[&str] = &["compilation_unit", "block", "switch_section"];

impl Grammar for CSharp {
    fn containers(&self) -> &'static [&'static str] {
        CONTAINERS
    }

    fn is_body_member(&self, child: Node<'_>, _field: Option<&str>) -> bool {
        child.is_named()
            && child
                .parent()
                .is_none_or(|parent| parent.kind() != "switch_section" || is_section_body(child))
    }

    fn classify<'a>(&self, member: Node<'a>) -> Role<'a> {
        if member.kind() == "comment" {
            return Role::Comment;
        }
        Role::Statement(target_of(unwrap_wrappers(member)))
    }

    fn assigned_names(&self, statement: Node<'_>, source: &[u8]) -> Vec<String> {
        let node = unwrap_wrappers(statement);
        match node.kind() {
            "local_declaration_statement" => local_names(node, source),
            "expression_statement" => named_children(node)
                .into_iter()
                .next()
                .map_or_else(Vec::new, |expression| assignment_names(expression, source)),
            _ => Vec::new(),
        }
    }

    fn is_condition_name(&self, node: Node<'_>) -> bool {
        node.kind() == "identifier"
            && node.parent().is_none_or(|parent| {
                !(parent.kind() == "member_access_expression" && is_field_of(parent, "name", node))
            })
    }

    fn is_directive_line(&self, line: &str) -> bool {
        line.trim_start().starts_with('#')
    }
}

fn is_section_body(child: Node<'_>) -> bool {
    matches!(child.kind(), "comment" | "block") || child.kind().ends_with("_statement")
}

fn unwrap_wrappers(node: Node<'_>) -> Node<'_> {
    match node.kind() {
        "global_statement" => named_children(node)
            .into_iter()
            .next()
            .map_or(node, unwrap_wrappers),
        "labeled_statement" => named_children(node)
            .into_iter()
            .last()
            .map_or(node, unwrap_wrappers),
        _ => node,
    }
}

fn target_of(node: Node<'_>) -> Option<Target<'_>> {
    let target = match node.kind() {
        "if_statement" => Target {
            keyword: Keyword::If,
            written: "if",
            ends_with_block: ends_with_braced_body(node, "block", None),
            conditions: field_nodes(node, "condition"),
        },
        "for_statement" => body_target(node, Keyword::For, "for", "block"),
        "foreach_statement" => body_target(node, Keyword::For, "foreach", "block"),
        "while_statement" => body_target(node, Keyword::While, "while", "block"),
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
        "try_statement" => block_target(Keyword::Try, "try"),
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

fn local_names(statement: Node<'_>, source: &[u8]) -> Vec<String> {
    named_children(statement)
        .into_iter()
        .filter(|child| child.kind() == "variable_declaration")
        .flat_map(|declaration| named_children(declaration))
        .filter(|child| child.kind() == "variable_declarator")
        .flat_map(|declarator| declarator_names(declarator, source))
        .collect()
}

fn declarator_names(declarator: Node<'_>, source: &[u8]) -> Vec<String> {
    match declarator.child_by_field_name("name") {
        Some(name) => vec![node_text(name, source)],
        None => named_children(declarator)
            .into_iter()
            .filter(|child| child.kind() == "tuple_pattern")
            .flat_map(|tuple| descendants_of_kind(tuple, &["identifier"]))
            .map(|identifier| node_text(identifier, source))
            .collect(),
    }
}

fn assignment_names(expression: Node<'_>, source: &[u8]) -> Vec<String> {
    if expression.kind() != "assignment_expression" {
        return Vec::new();
    }
    let Some(left) = expression.child_by_field_name("left") else {
        return Vec::new();
    };
    match left.kind() {
        "identifier" => vec![node_text(left, source)],
        "declaration_expression" => named_children(left)
            .into_iter()
            .filter(|child| !is_field_of(left, "type", *child))
            .flat_map(|child| descendants_of_kind(child, &["identifier"]))
            .map(|identifier| node_text(identifier, source))
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::parse_source;
    use std::path::Path;

    fn assigned_names_of(source: &str) -> Vec<String> {
        let tree = parse_source(Path::new("a.cs"), source).expect("source parses");
        let statement = tree
            .root_node()
            .named_child(0)
            .expect("one top-level statement");
        CSharp.assigned_names(statement, source.as_bytes())
    }

    #[test]
    fn var_declaration_names_its_target() {
        assert_eq!(assigned_names_of("var x = F();"), vec!["x".to_string()]);
    }

    #[test]
    fn declaration_with_two_declarators_names_both() {
        assert_eq!(
            assigned_names_of("int a = 1, b = 2;"),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn top_level_assignment_names_its_target() {
        assert_eq!(assigned_names_of("x = F();"), vec!["x".to_string()]);
    }

    #[test]
    fn compound_assignment_names_its_target() {
        assert_eq!(assigned_names_of("x += 1;"), vec!["x".to_string()]);
    }

    #[test]
    fn member_assignment_names_nothing() {
        assert!(assigned_names_of("this.x = F();").is_empty());
    }
}
