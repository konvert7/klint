mod common;
mod complexity_support;

use complexity_support::*;

#[test]
fn csharp_scoring_rows_nesting_and_names_have_the_exact_score() {
    assert_cases(cs_cases::CASES);
}

#[test]
fn csharp_function_kinds_are_scored() {
    assert_cases(cs_kinds::CASES);
}
