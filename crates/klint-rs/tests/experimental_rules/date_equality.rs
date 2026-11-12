use crate::support::*;

#[test]
fn date_equality_new_date_triple_equals_reports_on_its_line_with_operator() {
    let (row, control) = rule_run(
        DATE_RULE,
        DATE_CONTROL,
        "const r = new Date(1) === new Date(2);",
    );
    assert_eq!(lines(&row), [1], "{row:?}");
    assert!(row[0]["message"].as_str().expect("m").contains("`===`"));
    assert_eq!(lines(&control), [DATE_CONTROL_LINE]);
}

#[test]
fn date_equality_const_date_not_equal_reports_with_not_equal_operator() {
    let (row, control) = rule_run(
        DATE_RULE,
        DATE_CONTROL,
        "const a = new Date(1);\nconst r = a != new Date(2);",
    );
    assert_eq!(lines(&row), [2], "{row:?}");
    assert!(row[0]["message"].as_str().expect("m").contains("`!=`"));
    assert_eq!(lines(&control), [DATE_CONTROL_LINE]);
}

#[test]
fn date_equality_relational_comparison_is_silent() {
    expect_date("const r = new Date(1) <= new Date(2);", &[]);
}

#[test]
fn date_equality_comparison_with_null_is_silent() {
    expect_date("const a = new Date(1);\nconst r = a === null;", &[]);
}

#[test]
fn date_equality_call_results_are_silent() {
    expect_date("const r = now() === now();", &[]);
}

#[test]
fn date_equality_property_reads_are_silent() {
    expect_date("const r = a.created === b.created;", &[]);
}

#[test]
fn date_equality_parenthesised_double_equals_reports_with_double_equals() {
    let (row, _) = rule_run(
        DATE_RULE,
        DATE_CONTROL,
        "const r = (new Date(1)) == (new Date(2));",
    );
    assert_eq!(lines(&row), [1], "{row:?}");
    assert!(row[0]["message"].as_str().expect("m").contains("`==`"));
}

#[test]
fn const_date_same_file_is_provable() {
    expect_date("const a = new Date(1);\nconst r = a === a;", &[2]);
}

#[test]
fn const_date_with_date_annotation_is_provable() {
    expect_date(
        "const a: Date = new Date(1);\nconst r = a === new Date(9);",
        &[2],
    );
}

#[test]
fn const_with_non_date_annotation_is_not_provable() {
    expect_date(
        "const a: object = new Date(1);\nconst r = a === new Date(9);",
        &[],
    );
}

#[test]
fn let_date_is_not_provable() {
    expect_date("let a = new Date(1);\nconst r = a === new Date(9);", &[]);
}

#[test]
fn const_date_read_inside_a_function_is_provable() {
    expect_date(
        "const a = new Date(1);\nfunction f() { return a === new Date(9); }",
        &[2],
    );
}

#[test]
fn parameter_of_the_enclosing_function_shadows_the_const_date() {
    expect_date(
        "const a = new Date(1);\nfunction f(a: number) { return a === new Date(9); }",
        &[],
    );
}

#[test]
fn parameter_of_an_unrelated_function_does_not_shadow_the_const_date() {
    expect_date(
        "const a = new Date(1);\nfunction g(a: number) {}\nconst r = a === new Date(9);",
        &[3],
    );
}

#[test]
fn inner_block_const_does_not_shadow_a_read_outside_it() {
    expect_date(
        "const a = new Date(1);\nfunction f() { { const a = 5; } return a === new Date(9); }",
        &[2],
    );
}

#[test]
fn a_bound_date_name_makes_new_date_unprovable() {
    expect_date(
        "const Date = X;\nconst a = new Date(1);\nconst r = a === new Date(9);",
        &[],
    );
}

#[test]
fn const_date_used_outside_its_block_is_not_provable() {
    expect_date(
        "{\n  const a = new Date(1);\n}\nconst r = a === new Date(9);",
        &[],
    );
}

#[test]
fn parenthesised_identifier_resolves_like_the_identifier() {
    expect_date(
        "const a = new Date(1);\nconst r = (a) === new Date(9);",
        &[2],
    );
}

#[test]
fn nested_parentheses_around_new_date_are_stripped() {
    expect_date("const r = ((new Date(1))) === new Date(2);", &[1]);
}

#[test]
fn no_date_equality_run_reports_triple_equals_with_the_exact_message() {
    let (row, _) = rule_run(
        DATE_RULE,
        DATE_CONTROL,
        "const r = new Date(1) === new Date(2);",
    );
    assert_eq!(lines(&row), [1]);
    assert_eq!(
        row[0]["message"],
        "Comparing two Date objects with `===` compares identity, not time — compare `a.getTime()` values instead."
    );
}

#[test]
fn no_date_equality_run_is_silent_on_a_file_with_a_syntax_error() {
    expect_date("const r = new Date(1) === new Date(2);\nconst = ;\n", &[]);
}

#[test]
fn no_date_equality_is_an_experimental_rule_that_runs() {
    expect_date("const a = new Date(1);\nconst r = a === new Date(9);", &[2]);
}
