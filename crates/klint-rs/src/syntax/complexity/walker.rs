use tree_sitter::{Node, TreeCursor};

use super::FunctionScore;
use super::booleans::boolean_runs;
use super::grammar::{ComplexityGrammar, Construct};

#[derive(Clone, Copy)]
struct Context<'a> {
    nesting: u32,
    frame: Option<usize>,
    boolean_parent: bool,
    parent: Option<Node<'a>>,
}

const ROOT_CONTEXT: Context<'static> = Context {
    nesting: 0,
    frame: None,
    boolean_parent: false,
    parent: None,
};

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

fn visit<'a>(
    node: Node<'a>,
    context: Context<'a>,
    grammar: &dyn ComplexityGrammar,
    scores: &mut Vec<FunctionScore<'a>>,
) -> Context<'a> {
    let construct = construct_of(node, context.parent, grammar);
    if construct == Construct::Function {
        return enter_function(node, context, grammar, scores);
    }
    let Some(frame) = context.frame else {
        return child_context(node, context, grammar, 0, None);
    };
    let (points, nesting) = score_step(construct, context.nesting);
    scores[frame].score += points + boolean_points(node, context, grammar);
    child_context(node, context, grammar, nesting, Some(frame))
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
    grammar: &dyn ComplexityGrammar,
    scores: &mut Vec<FunctionScore<'a>>,
) -> Context<'a> {
    scores.push(FunctionScore { node, score: 0 });
    let nesting = if context.frame.is_none() {
        0
    } else {
        context.nesting + 1
    };
    child_context(node, context, grammar, nesting, Some(scores.len() - 1))
}

fn score_step(construct: Construct, nesting: u32) -> (u32, u32) {
    match construct {
        Construct::Plain | Construct::Function => (0, nesting),
        Construct::Nesting => (1 + nesting, nesting + 1),
        Construct::ChainedIf => (nesting, nesting),
        Construct::NestingOnly => (0, nesting + 1),
        Construct::Increment => (1, nesting),
        Construct::LoopElse => (1, nesting.saturating_sub(1)),
    }
}

fn boolean_points(node: Node<'_>, context: Context<'_>, grammar: &dyn ComplexityGrammar) -> u32 {
    if !context.boolean_parent && grammar.is_boolean_expression(node) {
        boolean_runs(node, grammar)
    } else {
        0
    }
}

fn child_context<'a>(
    node: Node<'a>,
    context: Context<'a>,
    grammar: &dyn ComplexityGrammar,
    nesting: u32,
    frame: Option<usize>,
) -> Context<'a> {
    let boolean_parent = grammar.is_boolean_expression(node)
        || (grammar.is_parenthesized(node) && context.boolean_parent);
    Context {
        nesting,
        frame,
        boolean_parent,
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
        assert_eq!(score_step(Construct::NestingOnly, 0), (0, 1));
        assert_eq!(score_step(Construct::Increment, 3), (1, 3));
        assert_eq!(score_step(Construct::LoopElse, 1), (1, 0));
        assert_eq!(score_step(Construct::Plain, 4), (0, 4));
    }
}
