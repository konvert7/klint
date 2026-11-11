use super::EdgeCase;

pub const CASES: [EdgeCase; 18] = [
    EdgeCase {
        name: "cs_comment_before_catch",
        file: "comment_before_catch.cs",
        input: "class C {
    void F() {
        try {
            A();
        }
        // handle
        catch (E e) {
            B();
        }
        finally {
            C();
        }
        D();
    }
}
",
        expected: Some(
            "class C {
    void F() {
        try {
            A();
        }
        // handle
        catch (E e) {
            B();
        }
        finally {
            C();
        }

        D();
    }
}
",
        ),
    },
    EdgeCase {
        name: "cs_stacked_comments",
        file: "stacked_comments.cs",
        input: "class C {
    void F() {
        A();
        // first
        /* second */
        foreach (var y in z) {
            B(y);
        }
        C();
    }
}
",
        expected: Some(
            "class C {
    void F() {
        A();

        // first
        /* second */
        foreach (var y in z) {
            B(y);
        }

        C();
    }
}
",
        ),
    },
    EdgeCase {
        name: "cs_switch_section",
        file: "switch_section.cs",
        input: "class C {
    void F(int v) {
        switch (v) {
            case 1:
                A();
                if (v > 0) {
                    B();
                }
                break;
            default:
                // nothing
                return;
        }
    }
}
",
        expected: Some(
            "class C {
    void F(int v) {
        switch (v) {
            case 1:
                A();

                if (v > 0) {
                    B();
                }

                break;
            default:
                // nothing
                return;
        }
    }
}
",
        ),
    },
    EdgeCase {
        name: "cs_verbatim_string",
        file: "verbatim_string.cs",
        input: "class C {
    void F() {
        var s = @\"
if (x) {
    return;
}
\";
        G(s);
    }
}
",
        expected: None,
    },
    EdgeCase {
        name: "cs_xml_doc_and_region",
        file: "xml_doc_and_region.cs",
        input: "class C {
    int F() {
        A();
        #region checks
        if (x) {
            return 1;
        }
        #endregion
        return 2;
    }
}
",
        expected: None,
    },
    EdgeCase {
        name: "rs_doc_and_attr",
        file: "doc_and_attr.rs",
        input: "fn f() {
    let a = 1;
    // explain
    #[allow(unused_variables)]
    for y in z {
        g(y);
    }
    h(a);
}
",
        expected: Some(
            "fn f() {
    let a = 1;

    // explain
    #[allow(unused_variables)]
    for y in z {
        g(y);
    }

    h(a);
}
",
        ),
    },
    EdgeCase {
        name: "rs_last_comment_in_block",
        file: "last_comment_in_block.rs",
        input: "fn f() {
    if x {
        a();
        // trailing thought
    }
    b();
}
",
        expected: Some(
            "fn f() {
    if x {
        a();
        // trailing thought
    }

    b();
}
",
        ),
    },
    EdgeCase {
        name: "rs_match_arms",
        file: "match_arms.rs",
        input: "fn f(v: Option<i32>) -> i32 {
    match v {
        Some(n) => {
            let m = n + 1;
            if m > 2 {
                return m;
            }
            m
        }
        None => 0,
    }
}
",
        expected: Some(
            "fn f(v: Option<i32>) -> i32 {
    match v {
        Some(n) => {
            let m = n + 1;
            if m > 2 {
                return m;
            }

            m
        }
        None => 0,
    }
}
",
        ),
    },
    EdgeCase {
        name: "rs_raw_string",
        file: "raw_string.rs",
        input: "fn f() {
    let s = r\"
if x {
    return;
}
\";
    g(s);
}
",
        expected: None,
    },
    EdgeCase {
        name: "rs_setup_comment",
        file: "setup_comment.rs",
        input: "fn f() -> i32 {
    let x = g();
    // check it
    if x > 1 {
        return 1;
    }
    x
}
",
        expected: Some(
            "fn f() -> i32 {
    let x = g();
    // check it
    if x > 1 {
        return 1;
    }

    x
}
",
        ),
    },
    EdgeCase {
        name: "rs_trailing_comment_prev",
        file: "trailing_comment_prev.rs",
        input: "fn f() -> i32 {
    a(); // note
    if x {
        b();
    } // end
    c();
    /* block */
    return 1;
}
",
        expected: Some(
            "fn f() -> i32 {
    a(); // note

    if x {
        b();
    } // end

    c();

    /* block */
    return 1;
}
",
        ),
    },
    EdgeCase {
        name: "sw_comment_before_catch",
        file: "comment_before_catch.swift",
        input: "func f() {
    do {
        try a()
    }
    // handle
    catch {
        b()
    }
    c()
}
",
        expected: Some(
            "func f() {
    do {
        try a()
    }
    // handle
    catch {
        b()
    }

    c()
}
",
        ),
    },
    EdgeCase {
        name: "sw_labeled_comment",
        file: "labeled_comment.swift",
        input: "func f() {
    a()
    // outer loop
    outer: for i in xs {
        b(i)
    }
    c()
}
",
        expected: Some(
            "func f() {
    a()

    // outer loop
    outer: for i in xs {
        b(i)
    }

    c()
}
",
        ),
    },
    EdgeCase {
        name: "sw_setup_comment",
        file: "setup_comment.swift",
        input: "func f() -> Int {
    let x = g()
    // check it
    if x > 1 {
        return 1
    }
    return x
}
",
        expected: Some(
            "func f() -> Int {
    let x = g()
    // check it
    if x > 1 {
        return 1
    }

    return x
}
",
        ),
    },
    EdgeCase {
        name: "sw_stacked_comments",
        file: "stacked_comments.swift",
        input: "func f() {
    a()
    // first
    /// doc-ish
    guard let y = z else {
        return
    }
    b(y)
}
",
        expected: Some(
            "func f() {
    a()

    // first
    /// doc-ish
    guard let y = z else {
        return
    }

    b(y)
}
",
        ),
    },
    EdgeCase {
        name: "sw_switch_case",
        file: "switch_case.swift",
        input: "func f(v: Int) {
    switch v {
    case 1:
        a()
        if v > 0 {
            b()
        }
    default:
        // nothing
        if v < 0 {
            c()
        }
    }
}
",
        expected: Some(
            "func f(v: Int) {
    switch v {
    case 1:
        a()

        if v > 0 {
            b()
        }
    default:
        // nothing
        if v < 0 {
            c()
        }
    }
}
",
        ),
    },
    EdgeCase {
        name: "cs_if_debug",
        file: "if_debug.cs",
        input: "class C {
    void F() {
        A();
#if DEBUG
        if (x) {
            Log();
        }
#endif
        B();
        #pragma warning disable CS0168
        foreach (var y in z) {
            G(y);
        }
    }
}
",
        expected: None,
    },
    EdgeCase {
        name: "sw_if_directive",
        file: "if_directive.swift",
        input: "func f() {
    a()
    #if DEBUG
    if x {
        log()
    }
    #endif
    b()
}
",
        expected: None,
    },
];
