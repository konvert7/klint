use super::grammar::{
    Grammar, Role, Target, block_target, field_nodes, is_field_of, named_children, node_text,
};
use super::keyword::Keyword;
use tree_sitter::Node;

pub(super) struct Python;

const CONTAINERS: &[&str] = &["module", "block"];

impl Grammar for Python {
    fn containers(&self) -> &'static [&'static str] {
        CONTAINERS
    }

    fn is_body_member(&self, child: Node<'_>, field: Option<&str>) -> bool {
        child.is_named() && field != Some("alternative")
    }

    fn classify<'a>(&self, member: Node<'a>) -> Role<'a> {
        if member.kind() == "comment" || is_docstring(member) {
            return Role::Comment;
        }
        Role::Statement(target_of(member))
    }

    fn assigned_names(&self, statement: Node<'_>, source: &[u8]) -> Vec<String> {
        if statement.kind() != "expression_statement" {
            return Vec::new();
        }
        named_children(statement)
            .into_iter()
            .flat_map(|expression| assignment_names(expression, source))
            .collect()
    }

    fn is_condition_name(&self, node: Node<'_>) -> bool {
        node.kind() == "identifier"
            && node
                .parent()
                .is_none_or(|parent| !is_member_or_keyword_name(parent, node))
    }
}

fn is_docstring(statement: Node<'_>) -> bool {
    statement.kind() == "expression_statement"
        && named_children(statement)
            .first()
            .is_some_and(|expression| expression.kind() == "string")
        && statement.named_child_count() == 1
        && only_comments_before(statement)
        && statement.parent().is_some_and(is_documented_body)
}

fn only_comments_before(statement: Node<'_>) -> bool {
    let mut sibling = statement.prev_named_sibling();
    while let Some(node) = sibling {
        if node.kind() != "comment" {
            return false;
        }
        sibling = node.prev_named_sibling();
    }
    true
}

fn is_documented_body(body: Node<'_>) -> bool {
    body.kind() == "module"
        || body
            .parent()
            .is_some_and(|owner| matches!(owner.kind(), "function_definition" | "class_definition"))
}

fn target_of(node: Node<'_>) -> Option<Target<'_>> {
    let target = match node.kind() {
        "if_statement" => Target {
            keyword: Keyword::If,
            written: "if",
            ends_with_block: true,
            conditions: field_nodes(node, "condition"),
        },
        "for_statement" => block_target(Keyword::For, "for"),
        "while_statement" => block_target(Keyword::While, "while"),
        "match_statement" => Target {
            keyword: Keyword::Switch,
            written: "match",
            ends_with_block: true,
            conditions: field_nodes(node, "subject"),
        },
        "try_statement" => block_target(Keyword::Try, "try"),
        "with_statement" => block_target(Keyword::With, "with"),
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

fn assignment_names(expression: Node<'_>, source: &[u8]) -> Vec<String> {
    match expression.kind() {
        "assignment" => assignment_targets(expression, source),
        "augmented_assignment" => expression
            .child_by_field_name("left")
            .filter(|left| left.kind() == "identifier")
            .map(|left| vec![node_text(left, source)])
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn assignment_targets(assignment: Node<'_>, source: &[u8]) -> Vec<String> {
    let mut names = assignment
        .child_by_field_name("left")
        .map_or_else(Vec::new, |left| target_names(left, source));
    if let Some(right) = assignment
        .child_by_field_name("right")
        .filter(|right| right.kind() == "assignment")
    {
        names.extend(assignment_names(right, source));
    }
    names
}

fn target_names(target: Node<'_>, source: &[u8]) -> Vec<String> {
    match target.kind() {
        "identifier" => vec![node_text(target, source)],
        "pattern_list" | "tuple_pattern" | "list_pattern" | "list_splat_pattern" => {
            named_children(target)
                .into_iter()
                .flat_map(|child| target_names(child, source))
                .collect()
        }
        _ => Vec::new(),
    }
}

fn is_member_or_keyword_name(parent: Node<'_>, node: Node<'_>) -> bool {
    match parent.kind() {
        "attribute" => is_field_of(parent, "attribute", node),
        "keyword_argument" => is_field_of(parent, "name", node),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::super::grammar::descendants_of_kind;
    use super::*;
    use crate::syntax::parse_source;
    use std::path::Path;

    fn assigned_names_of(source: &str) -> Vec<String> {
        let tree = parse_source(Path::new("a.py"), source).expect("source parses");
        let statement = descendants_of_kind(tree.root_node(), &["expression_statement"])
            .into_iter()
            .next()
            .expect("expression statement");
        Python.assigned_names(statement, source.as_bytes())
    }

    #[test]
    fn plain_assignment_names_its_target() {
        assert_eq!(assigned_names_of("x = f()"), vec!["x".to_string()]);
    }

    #[test]
    fn tuple_assignment_names_every_target() {
        assert_eq!(
            assigned_names_of("a, b = f()"),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn chained_assignment_names_every_target() {
        assert_eq!(
            assigned_names_of("a = b = f()"),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn augmented_assignment_names_its_target() {
        assert_eq!(assigned_names_of("x += 1"), vec!["x".to_string()]);
    }

    #[test]
    fn attribute_assignment_names_nothing() {
        assert!(assigned_names_of("self.x = f()").is_empty());
    }

    #[test]
    fn subscript_assignment_names_nothing() {
        assert!(assigned_names_of("d[k] = 1").is_empty());
    }
}
