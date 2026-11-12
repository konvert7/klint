mod common;
mod complexity_support;

use complexity_support::*;

#[test]
fn typescript_scoring_rows_have_the_exact_score() {
    assert_cases(ts_cases::CASES);
    assert_cases(ts_else_cases::CASES);
}

#[test]
fn typescript_function_names_follow_declared_assigned_property_and_anonymous() {
    assert_cases(ts_names::CASES);
}

#[test]
fn every_javascript_family_extension_is_scored_like_typescript() {
    for file in ts_names::SAME_SOURCE_EXTENSIONS {
        let fx = fixture(
            "cc-ext",
            &config_max(1),
            &[(file, ts_names::EXTENSION_SOURCE)],
        );
        let rows = reported(&lint(&fx));
        assert_eq!(rows, vec![(1, "f".to_string(), 3)], "{file}");
    }
}
