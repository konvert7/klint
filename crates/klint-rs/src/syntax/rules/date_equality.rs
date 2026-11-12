use tree_sitter::Node;

use super::{DateEqualityRecord, SameFileBindings, strip_parentheses};

const EQUALITY_OPERATORS: &[&str] = &["==", "===", "!=", "!=="];

pub(crate) fn date_equality_record<'a>(
    binary: Node<'a>,
    bindings: &SameFileBindings<'a>,
    source: &'a [u8],
) -> Option<DateEqualityRecord> {
    let operator = binary
        .child_by_field_name("operator")?
        .utf8_text(source)
        .ok()?;
    if !EQUALITY_OPERATORS.contains(&operator) {
        return None;
    }

    let left = strip_parentheses(binary.child_by_field_name("left")?);
    let right = strip_parentheses(binary.child_by_field_name("right")?);
    if !is_provable_date(left, bindings) || !is_provable_date(right, bindings) {
        return None;
    }

    Some(DateEqualityRecord {
        line: binary.start_position().row + 1,
        operator: operator.to_string(),
    })
}

fn is_provable_date<'a>(node: Node<'a>, bindings: &SameFileBindings<'a>) -> bool {
    bindings.is_new_date(node) || (node.kind() == "identifier" && bindings.is_const_date(node))
}
