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
        return enter_function(node, context, scores);
    }
    let Some(frame) = context.frame else {
        return child_context(node, 0, None);
    };
    let (points, nesting) = score_step(construct, context.nesting);
    scores[frame].score += points + boolean_points(node, context.parent, grammar);
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
    context: Context<'a>,
    scores: &mut Vec<FunctionScore<'a>>,
) -> Context<'a> {
    scores.push(FunctionScore { node, score: 0 });
    let nesting = if context.frame.is_none() {
        0
    } else {
        context.nesting + 1
    };
    child_context(node, nesting, Some(scores.len() - 1))
}

fn score_step(construct: Construct, nesting: u32) -> (u32, u32) {
    match construct {
        Construct::Plain | Construct::Function => (0, nesting),
        Construct::Nesting => (1 + nesting, nesting + 1),
        Construct::ChainedIf => (nesting, nesting),
        Construct::Increment => (1, nesting),
        Construct::LoopElse => (1, nesting.saturating_sub(1)),
    }
}

fn boolean_points(
    node: Node<'_>,
    parent: Option<Node<'_>>,
    grammar: &dyn ComplexityGrammar,
) -> u32 {
    grammar.boolean_run_points(node, parent).unwrap_or_else(|| {
        u32::from(grammar.is_boolean_expression(node) && !continues_run(node, parent, grammar))
    })
}

fn continues_run(
    node: Node<'_>,
    parent: Option<Node<'_>>,
    grammar: &dyn ComplexityGrammar,
) -> bool {
    parent.is_some_and(|parent| {
        grammar.is_boolean_expression(parent)
            && boolean_operator_of(parent, grammar) == boolean_operator_of(node, grammar)
    })
}

fn boolean_operator_of(node: Node<'_>, grammar: &dyn ComplexityGrammar) -> Option<BooleanOperator> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .filter(|child| !child.is_named())
        .find_map(|child| grammar.boolean_operator(child.kind()))
}

fn child_context(node: Node<'_>, nesting: u32, frame: Option<usize>) -> Context<'_> {
    Context {
        nesting,
        frame,
        parent: Some(node),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_construct_adds_its_points_and_sets_the_children_nesting() {
        assert_eq!(score_step(Construct::Nesting, 1), (2, 2));
        assert_eq!(score_step(Construct::ChainedIf, 2), (2, 2));
        assert_eq!(score_step(Construct::Increment, 3), (1, 3));
        assert_eq!(score_step(Construct::LoopElse, 1), (1, 0));
        assert_eq!(score_step(Construct::Plain, 4), (0, 4));
    }
}
