mod common;
mod complexity_support;

use complexity_support::*;

#[test]
fn python_scoring_rows_nesting_and_names_have_the_exact_score() {
    assert_cases(py_cases::CASES);
}

fn python_case(name: &str) -> &'static Case {
    py_cases::CASES
        .iter()
        .find(|case| case.name == name)
        .expect("named case is in py_cases")
}

#[test]
fn for_else_adds_one_and_its_body_stays_at_the_loop_level() {
    assert_case(python_case(
        "for else adds one and its body stays at the loop level",
    ));
}

#[test]
fn while_else_adds_one() {
    assert_case(python_case("while else adds one"));
}

#[test]
fn try_else_adds_one() {
    assert_case(python_case("try else adds one"));
}
