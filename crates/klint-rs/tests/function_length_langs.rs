mod common;
mod complexity_support;
mod function_length_support;

use function_length_support::*;

fn row(line: u64, name: &str, count: u64) -> (u64, String, u64) {
    (line, name.to_string(), count)
}

fn lines_and_counts(file: &str, source: &str) -> Vec<(u64, u64)> {
    rows_for(file, source, false)
        .into_iter()
        .map(|r| (r.0, r.2))
        .collect()
}

fn blank_line_pair(file: &str, source: &str) {
    expect_count(file, source, false, 3);
    expect_count(file, source, true, 2);
}

#[test]
fn python_counts_statements_and_skips_dict_braces() {
    expect_count("a.py", "def f():\n    a\n    b\n", false, 2);
    let source = "def f():\n    d = {\n        \"a\": 1,\n    }\n    g()\n";
    expect_count("a.py", source, false, 3);
}

#[test]
fn python_comment_line_counts() {
    expect_count("a.py", "def f():\n    a\n    # c\n    b\n", false, 3);
}

#[test]
fn python_blank_line_counts_unless_skipped() {
    blank_line_pair("a.py", "def f():\n    a\n\n    b\n");
}

#[test]
fn python_blank_lines_after_the_body_do_not_count() {
    let source = "def f():\n    a\n    b\n\n\ndef g():\n    a\n    b\n";
    assert_eq!(
        rows_for("a.py", source, false),
        vec![row(1, "f", 2), row(6, "g", 2)]
    );
}

#[test]
fn python_multi_line_string_counts_once() {
    let source = "def f():\n    s = \"\"\"a\nb\nc\"\"\"\n    g()\n";
    expect_count("a.py", source, false, 2);
}

#[test]
fn python_nested_function_counts_in_both() {
    let source = "def outer():\n    def inner():\n        a\n        b\n";
    assert_eq!(
        rows_for("a.py", source, false),
        vec![row(1, "outer", 3), row(2, "inner", 2)]
    );
}

#[test]
fn python_decorator_and_def_line_are_outside_the_body() {
    let source = "@dec\ndef f():\n    a\n    b\n";
    assert_eq!(rows_for("a.py", source, false), vec![row(2, "f", 2)]);
}

#[test]
fn python_docstring_counts() {
    let source = "def f():\n    \"\"\"doc\"\"\"\n    a\n";
    expect_count("a.py", source, false, 2);
}

#[test]
fn python_lambda_is_checked_and_named_by_assignment() {
    let source = "f = lambda: (\n    1,\n    2,\n)\n";
    assert_eq!(rows_for("a.py", source, false), vec![row(1, "f", 3)]);
}

#[test]
fn rust_counts_statements_and_skips_struct_braces() {
    expect_count("a.rs", "fn f() {\n    a();\n    b();\n}\n", false, 2);
    let source = "fn f() {\n    let s = S {\n        a: 1,\n    };\n    g();\n}\n";
    expect_count("a.rs", source, false, 3);
}

#[test]
fn rust_comment_line_counts() {
    expect_count(
        "a.rs",
        "fn f() {\n    a();\n    // c\n    b();\n}\n",
        false,
        3,
    );
}

#[test]
fn rust_blank_line_counts_unless_skipped() {
    blank_line_pair("a.rs", "fn f() {\n    a();\n\n    b();\n}\n");
}

#[test]
fn rust_trailing_blank_lines_before_the_closing_brace_do_not_count() {
    let source = "fn f() {\n    a();\n    b();\n\n\n}\n";
    expect_count("a.rs", source, false, 2);
}

#[test]
fn rust_multi_line_string_counts_once() {
    let source = "fn f() {\n    let s = \"a\nb\nc\";\n    g();\n}\n";
    expect_count("a.rs", source, false, 2);
}

#[test]
fn rust_nested_closure_counts_in_both() {
    let source = "fn outer() {\n    let inner = || {\n        a();\n        b();\n    };\n}\n";
    assert_eq!(
        rows_for("a.rs", source, false),
        vec![row(1, "outer", 3), row(2, "inner", 2)]
    );
}

#[test]
fn rust_attribute_is_outside_the_body() {
    let source = "#[inline]\nfn f() {\n    a();\n    b();\n}\n";
    assert_eq!(rows_for("a.rs", source, false), vec![row(2, "f", 2)]);
}

#[test]
fn rust_trait_method_without_default_is_not_checked() {
    let source =
        "trait T {\n    fn m(&self);\n    fn d(&self) {\n        a();\n        b();\n    }\n}\n";
    assert_eq!(rows_for("a.rs", source, false), vec![row(3, "d", 2)]);
}

#[test]
fn swift_counts_statements_and_nested_closures() {
    expect_count("a.swift", "func f() {\n    a()\n    b()\n}\n", false, 2);
    let source = "func outer() {\n    let inner = {\n        a()\n        b()\n    }\n}\n";
    assert_eq!(lines_and_counts("a.swift", source), vec![(1, 3), (2, 2)]);
}

#[test]
fn swift_comment_line_counts() {
    expect_count(
        "a.swift",
        "func f() {\n    a()\n    // c\n    b()\n}\n",
        false,
        3,
    );
}

#[test]
fn swift_blank_line_counts_unless_skipped() {
    blank_line_pair("a.swift", "func f() {\n    a()\n\n    b()\n}\n");
}

#[test]
fn swift_trailing_blank_lines_before_the_closing_brace_do_not_count() {
    expect_count("a.swift", "func f() {\n    a()\n    b()\n\n\n}\n", false, 2);
}

#[test]
fn swift_multi_line_string_counts_once() {
    let source = "func f() {\n    let s = \"\"\"\n    a\n    b\n    \"\"\"\n    g()\n}\n";
    expect_count("a.swift", source, false, 2);
}

#[test]
fn swift_init_and_deinit_are_checked() {
    let source = "class C {\n    init() {\n        a()\n        b()\n    }\n    deinit {\n        a()\n        b()\n    }\n}\n";
    assert_eq!(lines_and_counts("a.swift", source), vec![(2, 2), (6, 2)]);
}

#[test]
fn swift_accessors_are_not_checked() {
    let source = "struct S {\n    var v: Int {\n        get {\n            a()\n            b()\n        }\n        set {\n            a()\n            b()\n        }\n    }\n    var w: Int {\n        a()\n        b()\n    }\n}\nfunc m() {\n    a()\n    b()\n}\n";
    assert_eq!(rows_for("a.swift", source, false), vec![row(17, "m", 2)]);
}

#[test]
fn swift_function_is_named_by_its_declaration() {
    let source = "func declared() {\n    a()\n    b()\n}\n";
    assert_eq!(
        rows_for("a.swift", source, false),
        vec![row(1, "declared", 2)]
    );
}

#[test]
fn csharp_counts_statements_and_skips_initializer_braces() {
    let source = "class C {\n    void F() {\n        A();\n        B();\n    }\n}\n";
    assert_eq!(rows_for("a.cs", source, false), vec![row(2, "F", 2)]);
    let init = "class C {\n    void F() {\n        var l = new List<int> {\n            1,\n            2,\n        };\n        G();\n    }\n}\n";
    assert_eq!(rows_for("a.cs", init, false), vec![row(2, "F", 4)]);
}

#[test]
fn csharp_comment_line_counts() {
    let source = "class C {\n    void F() {\n        A();\n        // c\n        B();\n    }\n}\n";
    assert_eq!(rows_for("a.cs", source, false), vec![row(2, "F", 3)]);
}

#[test]
fn csharp_blank_line_counts_unless_skipped() {
    let source = "class C {\n    void F() {\n        A();\n\n        B();\n    }\n}\n";
    assert_eq!(rows_for("a.cs", source, false), vec![row(2, "F", 3)]);
    assert_eq!(rows_for("a.cs", source, true), vec![row(2, "F", 2)]);
}

#[test]
fn csharp_trailing_blank_lines_before_the_closing_brace_do_not_count() {
    let source = "class C {\n    void F() {\n        A();\n        B();\n\n\n    }\n}\n";
    assert_eq!(rows_for("a.cs", source, false), vec![row(2, "F", 2)]);
}

#[test]
fn csharp_multi_line_string_counts_once() {
    let source =
        "class C {\n    void F() {\n        var s = @\"a\nb\nc\";\n        G();\n    }\n}\n";
    assert_eq!(rows_for("a.cs", source, false), vec![row(2, "F", 2)]);
}

#[test]
fn csharp_local_function_counts_in_both() {
    let source = "class C {\n    void Outer() {\n        void Inner() {\n            A();\n            B();\n        }\n    }\n}\n";
    assert_eq!(
        rows_for("a.cs", source, false),
        vec![row(2, "Outer", 3), row(3, "Inner", 2)]
    );
}

#[test]
fn csharp_accessors_are_not_checked() {
    let source = "class C {\n    int P {\n        get {\n            A();\n            B();\n        }\n        set {\n            A();\n            B();\n        }\n    }\n    void M() {\n        A();\n        B();\n    }\n}\n";
    assert_eq!(rows_for("a.cs", source, false), vec![row(12, "M", 2)]);
}

#[test]
fn csharp_expression_bodied_member_counts_one() {
    let source = "class C {\n    int F() =>\n        1;\n}\n";
    expect_not_reported("a.cs", source);
    let wrapped = "class C {\n    void W() {\n        int F() =>\n            1;\n    }\n}\n";
    assert_eq!(rows_for("a.cs", wrapped, false), vec![row(2, "W", 2)]);
}

#[test]
fn csharp_constructor_destructor_and_operator_are_checked() {
    let source = "class C {\n    C() {\n        A();\n        B();\n    }\n    ~C() {\n        A();\n        B();\n    }\n    public static C operator +(C a, C b) {\n        A();\n        B();\n        return a;\n    }\n}\n";
    let got: Vec<u64> = lines_and_counts("a.cs", source)
        .iter()
        .map(|r| r.0)
        .collect();
    assert_eq!(got, vec![2, 6, 10]);
}
