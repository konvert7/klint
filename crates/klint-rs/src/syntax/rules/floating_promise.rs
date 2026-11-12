use tree_sitter::Node;

use super::same_file_bindings::is_named_identifier;
use super::{FloatingPromiseRecord, SameFileBindings, strip_parentheses};

const PROMISE_STATICS: &[&str] = &["resolve", "reject", "all", "allSettled", "race", "any"];

pub(crate) fn floating_promise_record<'a>(
    statement: Node<'a>,
    bindings: &SameFileBindings<'a>,
    source: &'a [u8],
) -> Option<FloatingPromiseRecord> {
    let expression = strip_parentheses(first_named_child(statement)?);

    is_provable_promise(expression, bindings, source).then_some(FloatingPromiseRecord {
        line: statement.start_position().row + 1,
    })
}

fn first_named_child(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|child| child.kind() != "comment")
}

fn is_provable_promise<'a>(
    node: Node<'a>,
    bindings: &SameFileBindings<'a>,
    source: &'a [u8],
) -> bool {
    match node.kind() {
        "new_expression" => is_new_promise(node, bindings, source),
        "call_expression" if !has_optional_chain(node) => is_provable_call(node, bindings, source),
        _ => false,
    }
}

fn is_provable_call<'a>(call: Node<'a>, bindings: &SameFileBindings<'a>, source: &'a [u8]) -> bool {
    let Some(function) = call.child_by_field_name("function") else {
        return false;
    };

    match function.kind() {
        "identifier" => bindings.is_same_file_async_function(function),
        "member_expression" if !has_optional_chain(function) => {
            is_provable_member_call(call, function, bindings, source)
        }
        _ => false,
    }
}

fn is_provable_member_call<'a>(
    call: Node<'a>,
    member: Node<'a>,
    bindings: &SameFileBindings<'a>,
    source: &'a [u8],
) -> bool {
    let Some(object) = member.child_by_field_name("object").map(strip_parentheses) else {
        return false;
    };
    let Some(property) = member
        .child_by_field_name("property")
        .and_then(|property| property.utf8_text(source).ok())
    else {
        return false;
    };

    if is_promise_static(object, property, source) {
        return bindings.is_unbound_global("Promise");
    }

    match property {
        "then" => {
            then_has_no_rejection_handler(call) && is_provable_promise(object, bindings, source)
        }
        "finally" => is_provable_promise(object, bindings, source),
        _ => false,
    }
}

fn is_promise_static(object: Node<'_>, property: &str, source: &[u8]) -> bool {
    is_named_identifier(object, source, "Promise") && PROMISE_STATICS.contains(&property)
}

fn then_has_no_rejection_handler(call: Node<'_>) -> bool {
    let Some(arguments) = call.child_by_field_name("arguments") else {
        return false;
    };
    let mut cursor = arguments.walk();
    let named: Vec<Node<'_>> = arguments
        .named_children(&mut cursor)
        .filter(|child| child.kind() != "comment")
        .collect();

    named.len() <= 1 && named.iter().all(|child| child.kind() != "spread_element")
}

fn is_new_promise<'a>(node: Node<'a>, bindings: &SameFileBindings<'a>, source: &'a [u8]) -> bool {
    node.child_by_field_name("constructor")
        .is_some_and(|constructor| is_named_identifier(constructor, source, "Promise"))
        && bindings.is_unbound_global("Promise")
}

fn has_optional_chain(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .any(|child| matches!(child.kind(), "optional_chain" | "?."))
}
