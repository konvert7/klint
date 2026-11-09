use tree_sitter::Node;

use super::{AsyncPredicateRecord, raw_node_text};

const PREDICATE_METHODS: &[&str] = &["filter", "some", "every", "find", "findIndex"];

pub(crate) fn async_predicate_record(
    node: Node<'_>,
    source: &[u8],
) -> Option<AsyncPredicateRecord> {
    let function = node.child_by_field_name("function")?;
    if function.kind() != "member_expression" {
        return None;
    }
    let method = raw_node_text(function.child_by_field_name("property")?, source)?;
    if !PREDICATE_METHODS.contains(&method.as_str()) {
        return None;
    }

    let callback = first_argument(node.child_by_field_name("arguments")?)?;
    if !is_inline_async_function(callback) {
        return None;
    }
    Some(AsyncPredicateRecord {
        line: callback.start_position().row + 1,
        method,
    })
}

fn first_argument(arguments: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = arguments.walk();
    arguments
        .named_children(&mut cursor)
        .find(|child| child.kind() != "comment")
}

fn is_inline_async_function(node: Node<'_>) -> bool {
    if !matches!(
        node.kind(),
        "arrow_function" | "function_expression" | "function"
    ) {
        return false;
    }
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .any(|child| child.kind() == "async")
}
