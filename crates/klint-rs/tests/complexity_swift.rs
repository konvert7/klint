mod common;
mod complexity_support;

use complexity_support::*;

#[test]
fn swift_scoring_rows_nesting_and_names_have_the_exact_score() {
    assert_cases(swift_cases::CASES);
}

#[test]
fn swift_function_kinds_are_scored() {
    assert_cases(swift_kinds::CASES);
}

#[test]
fn swift_boolean_expression_scores_two_runs_whatever_the_tree_shape() {
    let source = "func f(_ a: Bool, _ b: Bool, _ c: Bool) -> Bool {\n    return a && b || c\n}\n";
    let fx = fixture("cc-swift-bool", &config_max(1), &[("a.swift", source)]);
    assert_eq!(reported(&lint(&fx)), vec![(1, "f".to_string(), 2)]);
}
