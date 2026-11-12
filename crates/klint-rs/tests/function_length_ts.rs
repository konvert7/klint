mod common;
mod complexity_support;
mod function_length_support;

use function_length_support::*;

#[test]
fn braces_lines_do_not_count() {
    expect_count("a.ts", "function f() {\n  a();\n  b();\n}\n", false, 2);
}

#[test]
fn a_comment_line_counts() {
    expect_count("a.ts", "function f() {\n  // c\n  a();\n}\n", false, 2);
}

#[test]
fn a_blank_line_counts_unless_skipped() {
    let source = "function f() {\n  a();\n\n  b();\n}\n";
    expect_count("a.ts", source, false, 3);
    expect_count("a.ts", source, true, 2);
}

#[test]
fn trailing_blank_lines_before_the_closing_brace_do_not_count() {
    expect_not_reported("a.ts", "function f() {\n  x(); // t\n\n\n}\n");
    let wrapped = "function w() {\n  function f() {\n    x(); // t\n\n\n  }\n}\n";
    assert_eq!(
        rows_for("a.ts", wrapped, false),
        vec![(1, "w".to_string(), 2)]
    );
}

#[test]
fn a_closing_brace_and_semicolon_line_does_not_count() {
    let source = "function f() {\n  const o = {\n    a: 1,\n  };\n  b();\n}\n";
    let got = counts_for("a.ts", source, false);
    assert_eq!(got, vec![3], "{source:?}");
}

#[test]
fn a_multi_line_template_literal_counts_once() {
    let source = "function f() {\n  const s = `a\nb\nc`;\n  g();\n}\n";
    expect_count("a.ts", source, false, 2);
    let wrapped = "function w() {\n  function f() {\n    const s = `a\nb\nc`;\n  }\n}\n";
    assert_eq!(
        rows_for("a.ts", wrapped, false),
        vec![(1, "w".to_string(), 2)]
    );
}

#[test]
fn a_multi_line_block_comment_text_counts_its_breaks_between_tokens_only() {
    let source = "function f() {\n  a();\n  /* x\n  y */\n  b();\n}\n";
    expect_count("a.ts", source, false, 3);
}

#[test]
fn a_nested_arrow_counts_toward_its_parent_and_is_checked_on_its_own() {
    let source = "function outer() {\n  const inner = () => {\n    a();\n    b();\n  };\n}\n";
    let rows = rows_for("a.ts", source, false);
    assert_eq!(
        rows,
        vec![(1, "outer".to_string(), 3), (2, "inner".to_string(), 2)]
    );
}

#[test]
fn an_expression_bodied_arrow_on_the_next_line_counts_one() {
    expect_not_reported("a.ts", "const f = (x) =>\n  x + 1;\n");
    let wrapped = "function w() {\n  const f = (x) =>\n    x + 1;\n}\n";
    assert_eq!(
        rows_for("a.ts", wrapped, false),
        vec![(1, "w".to_string(), 2)]
    );
}

#[test]
fn a_doubly_nested_function_is_checked_on_its_own_and_counted_by_each_parent() {
    let source = "function a() {\n  function b() {\n    function c() {\n      x();\n      y();\n    }\n  }\n}\n";
    let rows = rows_for("a.ts", source, false);
    assert_eq!(
        rows,
        vec![
            (1, "a".to_string(), 4),
            (2, "b".to_string(), 3),
            (3, "c".to_string(), 2),
        ]
    );
}

#[test]
fn object_braces_are_skipped_like_block_braces() {
    let source = "function f() {\n  return {\n    a: 1,\n    b: 2,\n  };\n}\n";
    let got = counts_for("a.ts", source, false);
    assert_eq!(got, vec![3], "{source:?}");
}

#[test]
fn empty_and_one_line_bodies_count_zero() {
    for source in [
        "function f() {}\n",
        "function f() {\n}\n",
        "function f() { return 1; }\n",
    ] {
        let probe = format!("{source}function g() {{\n  a();\n  b();\n}}\n");
        let got = rows_for("a.ts", &probe, false);
        assert_eq!(got.len(), 1, "{source:?}: {got:?}");
        assert_eq!(got[0].1, "g");
    }
}

#[test]
fn crlf_is_one_line_break() {
    let source = "function f() {\r\n  a();\r\n  b();\r\n}\r\n";
    expect_count("a.ts", source, false, 2);
}

#[test]
fn getters_and_setters_are_not_checked() {
    let source = "class C {\n  get g() {\n    a();\n    b();\n  }\n  set g(v) {\n    a();\n    b();\n  }\n  m() {\n    a();\n    b();\n  }\n}\n";
    assert_eq!(
        rows_for("a.ts", source, false),
        vec![(10, "m".to_string(), 2)]
    );
}

#[test]
fn methods_constructors_and_object_methods_are_checked() {
    let source = "class C {\n  constructor() {\n    a();\n    b();\n  }\n  m() {\n    a();\n    b();\n  }\n}\nconst o = {\n  n() {\n    a();\n    b();\n  },\n};\n";
    let rows = rows_for("a.ts", source, false);
    let lines: Vec<u64> = rows.iter().map(|r| r.0).collect();
    assert_eq!(lines, vec![2, 6, 12], "{rows:?}");
}

#[test]
fn bodyless_signatures_are_not_checked() {
    let source = "abstract class C {\n  abstract m(): void;\n}\ndeclare function f(): void;\nfunction g(a: string): void;\nfunction g(a: number): void;\nfunction g(a: any) {\n  x();\n  y();\n}\n";
    let rows = rows_for("a.ts", source, false);
    assert_eq!(rows, vec![(7, "g".to_string(), 2)]);
}

#[test]
fn function_expressions_and_generators_are_checked() {
    let source =
        "const a = function () {\n  x();\n  y();\n};\nfunction* g() {\n  x();\n  y();\n}\n";
    let lines: Vec<u64> = rows_for("a.ts", source, false)
        .iter()
        .map(|r| r.0)
        .collect();
    assert_eq!(lines, vec![1, 5]);
}

#[test]
fn names_follow_declared_assigned_property_and_anonymous() {
    let source = "function declared() {\n  x();\n  y();\n}\nconst assigned = () => {\n  x();\n  y();\n};\nconst o = {\n  prop: function () {\n    x();\n    y();\n  },\n};\nexport default function () {\n  x();\n  y();\n}\n";
    let names: Vec<String> = rows_for("a.ts", source, false)
        .into_iter()
        .map(|r| r.1)
        .collect();
    assert_eq!(names[0], "declared");
    assert_eq!(names[1], "assigned");
    assert_eq!(names[2], "prop");
    assert_eq!(names[3], "<anonymous>");
}

#[test]
fn every_javascript_family_extension_counts_like_typescript() {
    for file in ["a.js", "a.jsx", "a.tsx", "a.mts", "a.cts"] {
        expect_count(file, "function f() {\n  a();\n  b();\n}\n", false, 2);
    }
}

#[test]
fn jsx_braces_are_skipped() {
    let source = "function f() {\n  return (\n    <div>\n      {x}\n    </div>\n  );\n}\n";
    let got = counts_for("a.tsx", source, false);
    assert_eq!(got.len(), 1, "{got:?}");
}

#[test]
fn skip_blank_lines_counts_a_multi_break_gap_once() {
    let source = "function f() {\n  a();\n\n\n\n  b();\n}\n";
    expect_count("a.ts", source, false, 5);
    expect_count("a.ts", source, true, 2);
}
