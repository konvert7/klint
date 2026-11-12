use super::*;

#[test]
fn each_construct_adds_its_points_and_sets_the_children_nesting() {
    assert_eq!(score_step(Construct::Nesting, 1), (2, 2));
    assert_eq!(score_step(Construct::ChainedIf, 2), (1, 2));
    assert_eq!(score_step(Construct::Increment, 3), (1, 3));
    assert_eq!(score_step(Construct::LoopElse, 1), (1, 0));
    assert_eq!(score_step(Construct::Plain, 4), (0, 4));
}

use crate::syntax::SourceLanguage;
use crate::syntax::complexity::grammar::BooleanOperator::{And, Nullish, Or};
use crate::syntax::complexity::grammar::complexity_grammar_for;
use crate::syntax::complexity::grammar::testing::{all_of_kind, parse};
use crate::syntax::complexity::score_functions;
use tree_sitter::Tree;

fn js_grammar() -> &'static dyn ComplexityGrammar {
    complexity_grammar_for(SourceLanguage::JavaScriptLike)
}

fn context_under(nesting: u32, parent: Node<'_>) -> Context<'_> {
    Context {
        nesting,
        frame: None,
        parent: Some(parent),
    }
}

fn first_binary(tree: &Tree) -> Node<'_> {
    all_of_kind(tree.root_node(), "binary_expression")[0]
}

fn score_of_first_function(source: &str, kind: &str) -> u32 {
    let tree = parse("a.ts", source);
    let function = all_of_kind(tree.root_node(), kind)[0];
    let scores = score_functions(&[tree.root_node()], SourceLanguage::JavaScriptLike);
    scores
        .iter()
        .find(|score| score.node.id() == function.id())
        .map(|score| score.score)
        .expect("the function is scored")
}

fn is_branch_in(path: &str, source: &str, kind: &str, language: SourceLanguage) -> bool {
    let tree = parse(path, source);
    let function = all_of_kind(tree.root_node(), kind)[0];
    is_nesting_branch(function, complexity_grammar_for(language))
}

#[test]
fn chained_if_at_nesting_0_adds_one_point_and_keeps_nesting_0() {
    assert_eq!(score_step(Construct::ChainedIf, 0), (1, 0));
}

#[test]
fn chained_if_at_nesting_2_adds_one_point_and_keeps_nesting_2() {
    assert_eq!(score_step(Construct::ChainedIf, 2), (1, 2));
}

#[test]
fn nesting_construct_at_nesting_1_adds_2_and_nests_its_body_to_2() {
    assert_eq!(score_step(Construct::Nesting, 1), (2, 2));
}

#[test]
fn increment_at_nesting_3_adds_1_and_keeps_nesting_3() {
    assert_eq!(score_step(Construct::Increment, 3), (1, 3));
}

#[test]
fn loop_else_at_nesting_1_adds_1_and_drops_to_0() {
    assert_eq!(score_step(Construct::LoopElse, 1), (1, 0));
}

#[test]
fn plain_at_nesting_4_adds_nothing_and_keeps_nesting_4() {
    assert_eq!(score_step(Construct::Plain, 4), (0, 4));
}

#[test]
fn top_level_function_starts_at_nesting_0() {
    assert_eq!(
        score_of_first_function(
            "function f() { if (a) { if (b) {} } }",
            "function_declaration"
        ),
        3
    );
}

#[test]
fn arrow_directly_in_a_function_body_starts_at_nesting_0() {
    assert_eq!(
        score_of_first_function(
            "function f(a) { const g = () => { if (a) { if (a) {} } }; }",
            "arrow_function"
        ),
        3
    );
}

#[test]
fn closure_inside_an_if_starts_at_nesting_0() {
    assert_eq!(
        score_of_first_function(
            "function f(a) { if (a) { const g = () => { if (a) { if (a) {} } }; } }",
            "arrow_function"
        ),
        3
    );
}

#[test]
fn method_of_a_class_inside_an_if_starts_at_nesting_0() {
    assert_eq!(
        score_of_first_function(
            "function f(a) { if (a) { class C { m() { if (a) { if (a) {} } } } } }",
            "method_definition"
        ),
        3
    );
}

#[test]
fn and_run_count_of_no_operators_is_0() {
    assert_eq!(and_run_count(&[]), 0);
}

#[test]
fn and_run_count_of_one_and_is_1() {
    assert_eq!(and_run_count(&[And]), 1);
}

#[test]
fn and_run_count_of_and_and_or_is_1() {
    assert_eq!(and_run_count(&[And, And, Or]), 1);
}

#[test]
fn and_run_count_of_and_or_and_is_2() {
    assert_eq!(and_run_count(&[And, Or, And]), 2);
}

#[test]
fn and_run_count_of_nullish_and_or_is_1() {
    assert_eq!(and_run_count(&[Nullish, And, Or]), 1);
}

#[test]
fn and_run_count_of_or_or_is_0() {
    assert_eq!(and_run_count(&[Or, Or]), 0);
}

#[test]
fn and_run_count_of_and_nullish_and_and_is_2() {
    assert_eq!(and_run_count(&[And, Nullish, And, And]), 2);
}

#[test]
fn sequence_of_and_and_or_reads_in_source_order() {
    let tree = parse("a.ts", "a && b && c || d;");
    assert_eq!(
        sequence_operators(first_binary(&tree), js_grammar()),
        vec![And, And, Or]
    );
}

#[test]
fn parenthesized_and_keeps_the_same_sequence() {
    let tree = parse("a.ts", "a && (b && c);");
    assert_eq!(
        sequence_operators(first_binary(&tree), js_grammar()),
        vec![And, And]
    );
}

#[test]
fn negated_group_is_not_part_of_the_sequence() {
    let tree = parse("a.ts", "a && !(b && c);");
    assert_eq!(
        sequence_operators(first_binary(&tree), js_grammar()),
        vec![And]
    );
}

#[test]
fn nullish_inside_parentheses_leads_the_sequence() {
    let tree = parse("a.ts", "(a ?? b) && c;");
    assert_eq!(
        sequence_operators(first_binary(&tree), js_grammar()),
        vec![Nullish, And]
    );
}

#[test]
fn and_returned_from_a_function_starts_a_sequence() {
    let tree = parse("a.ts", "function f(a, b) { return a && b; }");
    let node = first_binary(&tree);
    assert!(starts_boolean_sequence(node, node.parent(), js_grammar()));
}

#[test]
fn or_in_parentheses_under_and_does_not_start_a_sequence() {
    let tree = parse("a.ts", "a && (b || c);");
    let node = all_of_kind(tree.root_node(), "binary_expression")[1];
    assert!(!starts_boolean_sequence(node, node.parent(), js_grammar()));
}

#[test]
fn and_inside_a_negation_starts_a_sequence() {
    let tree = parse("a.ts", "!(b && c);");
    let node = first_binary(&tree);
    assert!(starts_boolean_sequence(node, node.parent(), js_grammar()));
}

#[test]
fn addition_does_not_start_a_sequence() {
    let tree = parse("a.ts", "a + b;");
    let node = first_binary(&tree);
    assert!(!starts_boolean_sequence(node, node.parent(), js_grammar()));
}

#[test]
fn ternary_test_of_an_if_drops_to_the_if_nesting() {
    let tree = parse("a.ts", "if (a ? b : c) {}");
    let if_statement = all_of_kind(tree.root_node(), "if_statement")[0];
    let test = if_statement
        .child_by_field_name("condition")
        .expect("if condition");
    assert_eq!(
        node_nesting(test, context_under(1, if_statement), js_grammar()),
        0
    );
}

#[test]
fn statement_in_an_if_body_keeps_the_body_nesting() {
    let tree = parse("a.ts", "if (a ? b : c) { g(); }");
    let statement = all_of_kind(tree.root_node(), "expression_statement")[0];
    let body = statement.parent().expect("statement block");
    assert_eq!(
        node_nesting(statement, context_under(1, body), js_grammar()),
        1
    );
}

#[test]
fn ternary_test_of_a_nested_if_drops_one_level() {
    let tree = parse("a.ts", "if (a) { if (b ? c : a) {} }");
    let inner = all_of_kind(tree.root_node(), "if_statement")[1];
    let test = inner
        .child_by_field_name("condition")
        .expect("inner condition");
    assert_eq!(node_nesting(test, context_under(2, inner), js_grammar()), 1);
}

#[test]
fn parenthesized_test_of_a_python_conditional_drops_to_its_nesting() {
    let tree = parse("a.py", "1 if (b if a else c) else 2\n");
    let conditional = all_of_kind(tree.root_node(), "conditional_expression")[0];
    let test = conditional.named_child(1).expect("the test");
    assert_eq!(
        node_nesting(
            test,
            context_under(1, conditional),
            complexity_grammar_for(SourceLanguage::Python)
        ),
        0
    );
}

#[test]
fn subject_of_a_csharp_switch_expression_drops_to_its_nesting() {
    let tree = parse(
        "a.cs",
        "class C { void M(bool a, bool b, bool c) { var s = (a ? b : c) switch { true => 1, _ => 2 }; } }",
    );
    let switch = all_of_kind(tree.root_node(), "switch_expression")[0];
    let subject = switch.named_child(0).expect("the subject");
    assert_eq!(
        node_nesting(
            subject,
            context_under(1, switch),
            complexity_grammar_for(SourceLanguage::CSharp)
        ),
        0
    );
}

#[test]
fn node_under_an_unlisted_parent_keeps_its_nesting() {
    let tree = parse("a.ts", "a + b;");
    let operand = all_of_kind(tree.root_node(), "identifier")[0];
    let sum = operand.parent().expect("binary expression");
    assert_eq!(
        node_nesting(operand, context_under(3, sum), js_grammar()),
        3
    );
}

#[test]
fn nesting_construct_with_no_leaked_nesting_adds_nothing() {
    assert_eq!(leaked_points(Construct::Nesting, 0), 0);
}

#[test]
fn nesting_construct_adds_one_per_leaked_nesting() {
    assert_eq!(leaked_points(Construct::Nesting, 1), 1);
}

#[test]
fn chained_if_adds_nothing_even_with_leaked_nesting() {
    assert_eq!(leaked_points(Construct::ChainedIf, 1), 0);
}

#[test]
fn increment_adds_nothing_even_with_leaked_nesting() {
    assert_eq!(leaked_points(Construct::Increment, 2), 0);
}

#[test]
fn arrow_as_the_consequence_of_a_ternary_is_a_nesting_branch() {
    assert!(is_branch_in(
        "a.ts",
        "a ? () => {} : b;",
        "arrow_function",
        SourceLanguage::JavaScriptLike
    ));
}

#[test]
fn function_expression_as_the_alternative_of_a_ternary_is_a_nesting_branch() {
    assert!(is_branch_in(
        "a.ts",
        "a ? b : function () {};",
        "function_expression",
        SourceLanguage::JavaScriptLike
    ));
}

#[test]
fn parenthesized_arrow_as_the_consequence_of_a_ternary_is_a_nesting_branch() {
    assert!(is_branch_in(
        "a.ts",
        "a ? (() => {}) : b;",
        "arrow_function",
        SourceLanguage::JavaScriptLike
    ));
}

#[test]
fn arrow_as_the_test_of_a_ternary_is_not_a_nesting_branch() {
    assert!(!is_branch_in(
        "a.ts",
        "(() => {}) ? a : b;",
        "arrow_function",
        SourceLanguage::JavaScriptLike
    ));
}

#[test]
fn arrow_in_a_declaration_is_not_a_nesting_branch() {
    assert!(!is_branch_in(
        "a.ts",
        "const f = () => {};",
        "arrow_function",
        SourceLanguage::JavaScriptLike
    ));
}

#[test]
fn parenthesized_lambda_as_the_branch_of_a_python_conditional_is_a_nesting_branch() {
    assert!(is_branch_in(
        "a.py",
        "(lambda: 1) if a else None\n",
        "lambda",
        SourceLanguage::Python
    ));
}

#[test]
fn conditional_inside_a_python_lambda_is_not_a_nesting_branch() {
    assert!(!is_branch_in(
        "a.py",
        "lambda: 1 if a else 2\n",
        "lambda",
        SourceLanguage::Python
    ));
}

#[test]
fn first_closure_of_a_swift_ternary_is_a_nesting_branch() {
    assert!(is_branch_in(
        "a.swift",
        "let g = a ? { 1 } : { 2 }\n",
        "lambda_literal",
        SourceLanguage::Swift
    ));
}

#[test]
fn lambda_as_the_branch_of_a_csharp_conditional_is_a_nesting_branch() {
    assert!(is_branch_in(
        "a.cs",
        "class C { void M(bool a) { System.Func<int> g = a ? () => 1 : null; } }",
        "lambda_expression",
        SourceLanguage::CSharp
    ));
}

#[test]
fn closure_in_a_rust_if_branch_is_not_a_nesting_branch() {
    assert!(!is_branch_in(
        "a.rs",
        "fn f(a: bool) { let g = if a { || 1 } else { || 2 }; }",
        "closure_expression",
        SourceLanguage::Rust
    ));
}
