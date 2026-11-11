use super::grammar::{
    Grammar, Role, Target, block_target, descendants_of_kind, field_nodes, named_children,
    node_text,
};
use super::keyword::Keyword;
use tree_sitter::Node;

pub(super) struct Swift;

const CONTAINERS: &[&str] = &["source_file", "statements"];

impl Grammar for Swift {
    fn containers(&self) -> &'static [&'static str] {
        CONTAINERS
    }

    fn is_body_member(&self, child: Node<'_>, _field: Option<&str>) -> bool {
        child.is_named()
    }

    fn classify<'a>(&self, member: Node<'a>) -> Role<'a> {
        match member.kind() {
            "comment" | "multiline_comment" => Role::Comment,
            "statement_label" => Role::Prefix,
            _ => Role::Statement(target_of(member)),
        }
    }

    fn assigned_names(&self, statement: Node<'_>, source: &[u8]) -> Vec<String> {
        match statement.kind() {
            "property_declaration" => property_names(statement, source),
            "assignment" => assigned_plain_target(statement, source),
            _ => Vec::new(),
        }
    }

    fn is_condition_name(&self, node: Node<'_>) -> bool {
        node.kind() == "simple_identifier"
            && node.parent().is_none_or(|parent| {
                !matches!(parent.kind(), "navigation_suffix" | "value_argument_label")
            })
    }

    fn is_directive_line(&self, line: &str) -> bool {
        let word = line.split_whitespace().next().unwrap_or_default();
        matches!(word, "#if" | "#elseif" | "#else" | "#endif")
    }
}

fn target_of(node: Node<'_>) -> Option<Target<'_>> {
    let target = match node.kind() {
        "if_statement" => Target {
            keyword: Keyword::If,
            written: "if",
            ends_with_block: true,
            conditions: field_nodes(node, "condition"),
        },
        "guard_statement" => Target {
            keyword: Keyword::Guard,
            written: "guard",
            ends_with_block: true,
            conditions: field_nodes(node, "condition"),
        },
        "for_statement" => block_target(Keyword::For, "for"),
        "while_statement" => block_target(Keyword::While, "while"),
        "repeat_while_statement" => Target {
            keyword: Keyword::While,
            written: "repeat",
            ends_with_block: false,
            conditions: Vec::new(),
        },
        "switch_statement" => Target {
            keyword: Keyword::Switch,
            written: "switch",
            ends_with_block: true,
            conditions: field_nodes(node, "expr"),
        },
        "do_statement" => block_target(Keyword::Try, "do"),
        "control_transfer_statement" if starts_with_return(node) => Target {
            keyword: Keyword::Return,
            written: "return",
            ends_with_block: false,
            conditions: Vec::new(),
        },
        _ => return None,
    };
    Some(target)
}

fn starts_with_return(node: Node<'_>) -> bool {
    node.child(0).is_some_and(|first| first.kind() == "return")
}

fn property_names(declaration: Node<'_>, source: &[u8]) -> Vec<String> {
    field_nodes(declaration, "name")
        .into_iter()
        .flat_map(|pattern| descendants_of_kind(pattern, &["simple_identifier"]))
        .map(|identifier| node_text(identifier, source))
        .collect()
}

fn assigned_plain_target(assignment: Node<'_>, source: &[u8]) -> Vec<String> {
    let Some(target) = assignment.child_by_field_name("target") else {
        return Vec::new();
    };
    let children = named_children(target);
    match children.as_slice() {
        [only] if only.kind() == "simple_identifier" => vec![node_text(*only, source)],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::grammar::descendants_of_kind;
    use super::*;
    use crate::syntax::parse_source;
    use std::path::Path;

    fn assigned_names_of(source: &str, statement_kind: &str) -> Vec<String> {
        let tree = parse_source(Path::new("a.swift"), source).expect("source parses");
        let statement = descendants_of_kind(tree.root_node(), &[statement_kind])
            .into_iter()
            .next()
            .expect("statement of the given kind");
        Swift.assigned_names(statement, source.as_bytes())
    }

    #[test]
    fn let_binding_names_its_target() {
        assert_eq!(
            assigned_names_of("let x = f()", "property_declaration"),
            vec!["x".to_string()]
        );
    }

    #[test]
    fn tuple_binding_names_every_element() {
        assert_eq!(
            assigned_names_of("let (a, b) = f()", "property_declaration"),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn typed_var_binding_names_its_target() {
        assert_eq!(
            assigned_names_of("var x: Int = 1", "property_declaration"),
            vec!["x".to_string()]
        );
    }

    #[test]
    fn plain_assignment_names_its_target() {
        assert_eq!(
            assigned_names_of("x = f()", "assignment"),
            vec!["x".to_string()]
        );
    }

    #[test]
    fn member_assignment_names_nothing() {
        assert!(assigned_names_of("self.x = f()", "assignment").is_empty());
    }
}
