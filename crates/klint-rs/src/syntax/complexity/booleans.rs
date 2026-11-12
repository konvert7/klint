use super::grammar::{BooleanOperator, ComplexityGrammar};
use tree_sitter::Node;

pub(super) fn boolean_runs(expression: Node<'_>, grammar: &dyn ComplexityGrammar) -> u32 {
    let mut operators = Vec::new();
    collect_operators(expression, grammar, &mut operators);
    count_runs(&operators)
}

fn collect_operators(
    node: Node<'_>,
    grammar: &dyn ComplexityGrammar,
    operators: &mut Vec<BooleanOperator>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if grammar.is_boolean_expression(child) || grammar.is_parenthesized(child) {
            collect_operators(child, grammar, operators);
        } else if let Some(operator) = operator_token(child, grammar) {
            operators.push(operator);
        }
    }
}

fn operator_token(child: Node<'_>, grammar: &dyn ComplexityGrammar) -> Option<BooleanOperator> {
    if child.is_named() {
        None
    } else {
        grammar.boolean_operator(child.kind())
    }
}

fn count_runs(operators: &[BooleanOperator]) -> u32 {
    let runs = operators
        .iter()
        .enumerate()
        .filter(|&(index, operator)| index == 0 || operators[index - 1] != *operator)
        .count();
    u32::try_from(runs).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use BooleanOperator::{And, Nullish, Or};

    #[test]
    fn runs_count_each_change_of_operator() {
        assert_eq!(count_runs(&[]), 0);
        assert_eq!(count_runs(&[And, And, Or]), 2);
        assert_eq!(count_runs(&[And, Or, And]), 3);
        assert_eq!(count_runs(&[Nullish, Nullish]), 1);
        assert_eq!(count_runs(&[Nullish, And]), 2);
    }
}
