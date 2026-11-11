mod blank_lines_support;
mod common;

use blank_lines_support::*;

fn check_every_case(cases: &[EdgeCase]) {
    for case in cases {
        let expected = case.expected.unwrap_or(case.input);
        let fx = fixture(case.name, &both(), &[(case.file, case.input)]);
        let first = fix(&fx);
        assert_eq!(text(&fx, case.file), expected, "{}: {first}", case.name);
        let second = fix(&fx);
        assert_eq!(text(&fx, case.file), expected, "{} second run", case.name);
        assert!(
            second.contains("applied 0 fix(es)"),
            "{}: {second}",
            case.name
        );
    }
}

#[test]
fn typescript_edge_cases_fix_exactly_and_stay_stable() {
    check_every_case(&edge_cases_ts::CASES);
}

#[test]
fn python_edge_cases_fix_exactly_and_stay_stable() {
    check_every_case(&edge_cases_py::CASES);
}

#[test]
fn rust_swift_and_csharp_edge_cases_fix_exactly_and_stay_stable() {
    check_every_case(&edge_cases_rs_sw_cs::CASES);
}
