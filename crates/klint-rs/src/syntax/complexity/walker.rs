use tree_sitter::Node;
#[cfg(test)]
use tree_sitter::TreeCursor;

use super::FunctionScore;
use super::grammar::{BooleanOperator, ComplexityGrammar, Construct};

#[derive(Clone, Copy, Default)]
pub(crate) struct Context<'a> {
    nesting: u32,
    frame: Option<usize>,
    parent: Option<Node<'a>>,
}

#[cfg(test)]
const ROOT_CONTEXT: Context<'static> = Context {
    nesting: 0,
    frame: None,
    parent: None,
};

#[cfg(test)]
pub(super) fn score_root<'a>(
    root: Node<'a>,
    grammar: &dyn ComplexityGrammar,
    scores: &mut Vec<FunctionScore<'a>>,
) {
    let mut cursor = root.walk();
    let mut stack: Vec<Context<'a>> = Vec::new();
    let mut next = Some(ROOT_CONTEXT);
    while let Some(context) = next {
        let children = visit(cursor.node(), context, grammar, scores);
        next = if cursor.goto_first_child() {
            stack.push(children);
            Some(children)
        } else {
            next_sibling_context(&mut cursor, &mut stack)
        };
    }
}

#[cfg(test)]
fn next_sibling_context<'a>(
    cursor: &mut TreeCursor<'a>,
    stack: &mut Vec<Context<'a>>,
) -> Option<Context<'a>> {
    loop {
        if cursor.goto_next_sibling() {
            return stack.last().copied();
        }
        if !cursor.goto_parent() {
            return None;
        }
        stack.pop();
    }
}

pub(super) fn visit<'a>(
    node: Node<'a>,
    context: Context<'a>,
    grammar: &dyn ComplexityGrammar,
    scores: &mut Vec<FunctionScore<'a>>,
) -> Context<'a> {
    if node.child_count() == 0 && !grammar.scores_leaf_kind(node.kind()) {
        return child_context(node, context.nesting, context.frame);
    }
    let construct = construct_of(node, context.parent, grammar);
    if construct == Construct::Function {
        return enter_function(node, context.frame, grammar, scores);
    }
    let Some(frame) = context.frame else {
        return child_context(node, 0, None);
    };
    let (points, nesting) = score_step(construct, node_nesting(node, context, grammar));
    let leaked = leaked_points(construct, scores[frame].leaked_nesting);
    scores[frame].score += points + boolean_points(node, context.parent, grammar) + leaked;
    child_context(node, nesting, Some(frame))
}

fn construct_of(
    node: Node<'_>,
    parent: Option<Node<'_>>,
    grammar: &dyn ComplexityGrammar,
) -> Construct {
    if grammar.function_kinds().contains(&node.kind()) {
        Construct::Function
    } else {
        grammar.construct(node, parent)
    }
}

fn enter_function<'a>(
    node: Node<'a>,
    enclosing: Option<usize>,
    grammar: &dyn ComplexityGrammar,
    scores: &mut Vec<FunctionScore<'a>>,
) -> Context<'a> {
    leak_into_enclosing(node, enclosing, grammar, scores);
    scores.push(FunctionScore {
        node,
        score: 0,
        leaked_nesting: 0,
    });
    child_context(node, 0, Some(scores.len() - 1))
}

fn leak_into_enclosing(
    node: Node<'_>,
    enclosing: Option<usize>,
    grammar: &dyn ComplexityGrammar,
    scores: &mut [FunctionScore<'_>],
) {
    if let Some(frame) = enclosing.filter(|_| is_nesting_branch(node, grammar)) {
        scores[frame].leaked_nesting += 1;
    }
}

fn is_nesting_branch(function: Node<'_>, grammar: &dyn ComplexityGrammar) -> bool {
    let mut reached = function;
    let mut parent = function.parent();
    while let Some(ancestor) = parent.filter(|ancestor| grammar.is_parenthesized(*ancestor)) {
        reached = ancestor;
        parent = ancestor.parent();
    }
    parent.is_some_and(|parent| grammar.is_conditional_branch(reached, parent))
}

fn score_step(construct: Construct, nesting: u32) -> (u32, u32) {
    match construct {
        Construct::Plain | Construct::Function => (0, nesting),
        Construct::Nesting => (1 + nesting, nesting + 1),
        Construct::ChainedIf => (1, nesting),
        Construct::Increment => (1, nesting),
        Construct::LoopElse => (1, nesting.saturating_sub(1)),
    }
}

fn node_nesting(node: Node<'_>, context: Context<'_>, grammar: &dyn ComplexityGrammar) -> u32 {
    match context.parent {
        Some(parent) if grammar.is_head(node, parent) => context.nesting.saturating_sub(1),
        _ => context.nesting,
    }
}

fn leaked_points(construct: Construct, leaked_nesting: u32) -> u32 {
    if construct == Construct::Nesting {
        leaked_nesting
    } else {
        0
    }
}

fn boolean_points(
    node: Node<'_>,
    parent: Option<Node<'_>>,
    grammar: &dyn ComplexityGrammar,
) -> u32 {
    if !starts_boolean_sequence(node, parent, grammar) {
        return 0;
    }
    let operators = sequence_operators(node, grammar);
    if grammar.skips_sequence(node, enclosing_expression(parent, grammar), &operators) {
        0
    } else {
        and_run_count(&operators)
    }
}

fn starts_boolean_sequence(
    node: Node<'_>,
    parent: Option<Node<'_>>,
    grammar: &dyn ComplexityGrammar,
) -> bool {
    grammar.is_boolean_expression(node)
        && !enclosing_expression(parent, grammar)
            .is_some_and(|enclosing| grammar.is_boolean_expression(enclosing))
}

fn enclosing_expression<'a>(
    parent: Option<Node<'a>>,
    grammar: &dyn ComplexityGrammar,
) -> Option<Node<'a>> {
    let mut current = parent;
    while let Some(ancestor) = current.filter(|ancestor| grammar.is_parenthesized(*ancestor)) {
        current = ancestor.parent();
    }
    current
}

fn sequence_operators(node: Node<'_>, grammar: &dyn ComplexityGrammar) -> Vec<BooleanOperator> {
    let mut operators = Vec::new();
    collect_sequence_operators(node, grammar, &mut operators);
    operators
}

fn collect_sequence_operators(
    node: Node<'_>,
    grammar: &dyn ComplexityGrammar,
    operators: &mut Vec<BooleanOperator>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if is_sequence_member(child, grammar) {
            collect_sequence_operators(child, grammar, operators);
        } else if !child.is_named() {
            operators.extend(grammar.boolean_operator(child.kind()));
        }
    }
}

fn is_sequence_member(node: Node<'_>, grammar: &dyn ComplexityGrammar) -> bool {
    grammar.is_boolean_expression(node) || grammar.is_parenthesized(node)
}

fn and_run_count(operators: &[BooleanOperator]) -> u32 {
    let runs = operators
        .iter()
        .enumerate()
        .filter(|&(index, &operator)| {
            operator == BooleanOperator::And
                && (index == 0 || operators[index - 1] != BooleanOperator::And)
        })
        .count();
    u32::try_from(runs).unwrap_or(u32::MAX)
}

fn child_context(node: Node<'_>, nesting: u32, frame: Option<usize>) -> Context<'_> {
    Context {
        nesting,
        frame,
        parent: Some(node),
    }
}

#[cfg(test)]
mod tests;
