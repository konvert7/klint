use super::Case;

pub const CASES: &[Case] = &[
    Case {
        name: "if else if else",
        file: "a.rs",
        source: r#"fn f(a: bool, b: bool) {
    if a {
        g();
    } else if b {
        g();
    } else {
        g();
    }
}
"#,
        expected: &[("fn f", "f", 3)],
    },
    Case {
        name: "else if inside for gets the nesting increment",
        file: "a.rs",
        source: r#"fn f(xs: Vec<i32>, a: bool, b: bool) {
    for x in xs {
        if a {
            g();
        } else if b {
            g();
        }
    }
}
"#,
        expected: &[("fn f", "f", 5)],
    },
    Case {
        name: "nested if",
        file: "a.rs",
        source: r#"fn f(a: bool, b: bool) {
    if a {
        if b {
            g();
        }
    }
}
"#,
        expected: &[("fn f", "f", 3)],
    },
    Case {
        name: "match",
        file: "a.rs",
        source: r#"fn f(a: i32) {
    match a {
        1 => g(),
        _ => h(),
    }
    if a > 0 {
        g();
    }
}
"#,
        expected: &[("fn f", "f", 2)],
    },
    Case {
        name: "match arm guard",
        file: "a.rs",
        source: r#"fn f(a: Option<i32>) {
    match a {
        Some(v) if v > 1 => g(),
        _ => h(),
    }
}
"#,
        expected: &[("fn f", "f", 2)],
    },
    Case {
        name: "if let and while let",
        file: "a.rs",
        source: r#"fn f(a: Option<i32>, mut v: Vec<i32>) {
    if let Some(x) = a {
        g(x);
    }
    while let Some(y) = v.pop() {
        g(y);
    }
}
"#,
        expected: &[("fn f", "f", 2)],
    },
    Case {
        name: "question mark adds nothing",
        file: "a.rs",
        source: r#"fn f(a: Option<i32>) -> Option<i32> {
    let x = a?;
    if x > 1 {
        g();
    }
    if x > 2 {
        g();
    }
    Some(x)
}
"#,
        expected: &[("fn f", "f", 2)],
    },
    Case {
        name: "loop for and while",
        file: "a.rs",
        source: r#"fn f(a: bool) {
    loop {
        break;
    }
    for _ in 0..3 {
        g();
    }
    while a {
        g();
    }
}
"#,
        expected: &[("fn f", "f", 3)],
    },
    Case {
        name: "labeled break",
        file: "a.rs",
        source: r#"fn f(xs: Vec<bool>, ys: Vec<bool>) {
    'outer: for x in xs {
        for y in &ys {
            if *y {
                break 'outer;
            }
        }
    }
}
"#,
        expected: &[("fn f", "f", 7)],
    },
    Case {
        name: "labeled continue",
        file: "a.rs",
        source: r#"fn f(xs: Vec<bool>) {
    'outer: for x in xs {
        if x {
            continue 'outer;
        }
    }
}
"#,
        expected: &[("fn f", "f", 4)],
    },
    Case {
        name: "unlabeled break and continue add nothing",
        file: "a.rs",
        source: r#"fn f(xs: Vec<bool>) {
    for x in xs {
        if x {
            break;
        }
        if x {
            continue;
        }
    }
}
"#,
        expected: &[("fn f", "f", 5)],
    },
    Case {
        name: "runs of boolean operators",
        file: "a.rs",
        source: r#"fn r1(a: bool, b: bool, c: bool, d: bool) -> bool {
    a && b && c || d
}
fn r2(a: bool, b: bool, c: bool) -> bool {
    a && (b || c)
}
fn r3(a: bool, b: bool, c: bool) -> bool {
    a && (b || c) && a
}
fn r5(a: bool, b: bool, c: bool) -> bool {
    a && (b && c)
}
fn r4(a: bool, b: bool) {
    if !a && !b {
        g();
    }
}
"#,
        expected: &[
            ("fn r1", "r1", 2),
            ("fn r2", "r2", 2),
            ("fn r3", "r3", 2),
            ("fn r4", "r4", 2),
            ("fn r5", "r5", 2),
        ],
    },
    Case {
        name: "closure inside an if inherits the nesting",
        file: "a.rs",
        source: r#"fn f(a: bool, b: bool, c: bool) {
    if a {
        let g = || {
            if b {
                if c {
                    h();
                }
            }
        };
        g();
    }
    if b {
        h();
    }
}
"#,
        expected: &[("fn f", "f", 2), ("let g = ||", "g", 7)],
    },
    Case {
        name: "anonymous closure passed as an argument",
        file: "a.rs",
        source: r#"fn f(xs: Vec<i32>, a: bool) {
    if a {
        g();
    }
    if a {
        g();
    }
    xs.iter().for_each(|x| {
        if *x > 1 {
            g();
        }
    });
}
"#,
        expected: &[("fn f", "f", 2), ("for_each(|x|", "<anonymous>", 2)],
    },
    Case {
        name: "impl methods start at nesting zero and bodiless trait methods are skipped",
        file: "a.rs",
        source: r#"trait T {
    fn sig(&self, a: bool);
}
fn empty() {}
impl A {
    fn m(&self, a: bool) {
        if a {
            if a {
                g();
            }
        }
    }
}
"#,
        expected: &[("fn m", "m", 3)],
    },
    Case {
        name: "two functions on the same line are both reported in column order",
        file: "a.rs",
        source: r#"fn a(x: bool) { if x { if x { g(); } } } fn b(y: bool) { if y { if y { if y { g(); } } } }
"#,
        expected: &[("fn a", "a", 3), ("fn a", "b", 6)],
    },
    Case {
        name: "let else at top level scores like guard",
        file: "a.rs",
        source: r#"fn lt(a: Option<i32>) {
    let Some(x) = a else {
        return;
    };
    if x > 1 {
        g();
    }
}
"#,
        expected: &[("fn lt", "lt", 2)],
    },
    Case {
        name: "let else nests its else block",
        file: "a.rs",
        source: r#"fn ln(a: Option<i32>, b: bool) {
    let Some(x) = a else {
        if b {
            h();
        }
        return;
    };
    g(x);
}
"#,
        expected: &[("fn ln", "ln", 3)],
    },
    Case {
        name: "let else inside a loop gets the nesting increment",
        file: "a.rs",
        source: r#"fn ll(xs: Vec<Option<i32>>) {
    for v in xs {
        let Some(x) = v else {
            continue;
        };
        g(x);
    }
}
"#,
        expected: &[("fn ll", "ll", 3)],
    },
];
