mod common;
mod complexity_support;

use complexity_support::*;

#[test]
fn rust_scoring_rows_nesting_and_names_have_the_exact_score() {
    assert_cases(rs_cases::CASES);
}

fn rust_case(name: &str) -> &'static Case {
    rs_cases::CASES
        .iter()
        .find(|case| case.name == name)
        .expect("named case is in rs_cases")
}

#[test]
fn let_else_at_top_level_scores_like_guard() {
    assert_case(rust_case("let else at top level scores like guard"));
}

#[test]
fn let_else_nests_its_else_block() {
    assert_case(rust_case("let else nests its else block"));
}

#[test]
fn let_else_inside_a_loop_gets_the_nesting_increment() {
    assert_case(rust_case(
        "let else inside a loop gets the nesting increment",
    ));
}
