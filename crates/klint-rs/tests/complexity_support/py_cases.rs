use super::Case;

pub const CASES: &[Case] = &[
    Case {
        name: "if elif else",
        file: "a.py",
        source: r#"def f(a, b):
    if a:
        g()
    elif b:
        g()
    else:
        g()
"#,
        expected: &[("def f", "f", 3)],
    },
    Case {
        name: "elif inside for adds one with no nesting increment",
        file: "a.py",
        source: r#"def f(xs, a, b):
    for x in xs:
        if a:
            g()
        elif b:
            g()
"#,
        expected: &[("def f", "f", 4)],
    },
    Case {
        name: "nested if",
        file: "a.py",
        source: r#"def f(a, b):
    if a:
        if b:
            g()
"#,
        expected: &[("def f", "f", 3)],
    },
    Case {
        name: "conditional expression",
        file: "a.py",
        source: r#"def f(a):
    if a:
        g()
    return 1 if a else 2
"#,
        expected: &[("def f", "f", 2)],
    },
    Case {
        name: "match",
        file: "a.py",
        source: r#"def f(a):
    match a:
        case 1:
            g()
        case _:
            h()
    if a:
        g()
"#,
        expected: &[("def f", "f", 2)],
    },
    Case {
        name: "for and while",
        file: "a.py",
        source: r#"def f(xs, a):
    for x in xs:
        g()
    while a:
        g()
"#,
        expected: &[("def f", "f", 2)],
    },
    Case {
        name: "except and finally",
        file: "a.py",
        source: r#"def f(a):
    try:
        g()
    except ValueError:
        if a:
            g()
    finally:
        if a:
            g()
"#,
        expected: &[("def f", "f", 4)],
    },
    Case {
        name: "finally adds nothing and its body is not nested",
        file: "a.py",
        source: r#"def e1(a):
    if a:
        g()
    try:
        g()
    finally:
        pass


def e2(a, b):
    try:
        g()
    finally:
        if a:
            if b:
                pass
"#,
        expected: &[("def e2", "e2", 3)],
    },
    Case {
        name: "and or runs and negation",
        file: "a.py",
        source: r#"def r1(a, b, c, d):
    if a:
        g()
    return a and b and c or d


def r2(a, b, c):
    if a:
        g()
    return a and (b or c)


def r3(a, b, c):
    if a:
        g()
    return a and (b or c) and a


def r5(a, b, c):
    if a:
        g()
    return a and (b and c)


def r6(a, b, c):
    if a:
        g()
    if b:
        g()
    return a or b or c


def r7(a, b, c):
    if a:
        g()
    return a and b or c and a


def r4(a, b):
    if not a and not b:
        g()
"#,
        expected: &[
            ("def r1", "r1", 2),
            ("def r2", "r2", 2),
            ("def r3", "r3", 3),
            ("def r5", "r5", 2),
            ("def r6", "r6", 2),
            ("def r7", "r7", 3),
            ("def r4", "r4", 2),
        ],
    },
    Case {
        name: "comprehension if clauses",
        file: "a.py",
        source: r#"def f(xs):
    if xs:
        g()
    return [x for x in xs if x if x > 1]
"#,
        expected: &[("def f", "f", 3)],
    },
    Case {
        name: "with and plain return add nothing",
        file: "a.py",
        source: r#"def f(a):
    with a:
        if a:
            g()
    if a:
        return f(a)
"#,
        expected: &[("def f", "f", 2)],
    },
    Case {
        name: "lambda assigned to a variable inside a function",
        file: "a.py",
        source: r#"def f():
    fn = lambda x: 1 if x else 2
    return fn
"#,
        expected: &[],
    },
    Case {
        name: "def nested in an if starts at nesting zero",
        file: "a.py",
        source: r#"def outer(a, b, c):
    if a:
        def inner():
            if b:
                if c:
                    g()
        inner()
    if b:
        g()
"#,
        expected: &[("def outer", "outer", 2), ("def inner", "inner", 3)],
    },
    Case {
        name: "method of a top-level class starts at nesting zero",
        file: "a.py",
        source: r#"class A:
    def m(self, a):
        if a:
            if a:
                g()
"#,
        expected: &[("def m", "m", 3)],
    },
    Case {
        name: "class declared in a function adds no nesting",
        file: "a.py",
        source: r#"def f(a):
    if a:
        class C:
            def m(self, b):
                if b:
                    g()
    if a:
        g()
"#,
        expected: &[("def f", "f", 2)],
    },
    Case {
        name: "anonymous lambda and empty function",
        file: "a.py",
        source: r#"def empty():
    pass


def f(xs):
    return sorted(xs, key=lambda x: 1 if x else 2)
"#,
        expected: &[],
    },
    Case {
        name: "two lambdas on one line are reported in column order",
        file: "a.py",
        source: r#"def f():
    return (lambda x: 1 if x else 2), (lambda y: y and y and 1 if y else 2)
"#,
        expected: &[("return (lambda", "<anonymous>", 2)],
    },
    Case {
        name: "module body is not scored",
        file: "a.py",
        source: r#"a = True
if a:
    if a:
        g()


def real(a):
    if a:
        g()
    if a:
        g()
"#,
        expected: &[("def real", "real", 2)],
    },
    Case {
        name: "for else adds one and its body stays at the loop level",
        file: "a.py",
        source: r#"def fe(xs, a):
    for x in xs:
        g()
    else:
        if a:
            g()
"#,
        expected: &[("def fe", "fe", 3)],
    },
    Case {
        name: "while else adds one",
        file: "a.py",
        source: r#"def we(a):
    while a:
        g()
    else:
        g()
"#,
        expected: &[("def we", "we", 2)],
    },
    Case {
        name: "try else adds one",
        file: "a.py",
        source: r#"def te(a):
    try:
        g()
    except ValueError:
        g()
    else:
        if a:
            g()
"#,
        expected: &[("def te", "te", 3)],
    },
    Case {
        name: "head constructs get no nesting increment but bodies do",
        file: "a.py",
        source: r#"def h1(a, b, c):
    if (b if a else c):
        g()


def h2(a, b, c, xs, ys):
    for x in (xs if a else ys):
        g()


def h3(a, b, c):
    while (b if a else c):
        g()


def h4(a, b, c):
    match (b if a else c):
        case 1:
            g()
        case _:
            h()


def h5(a, b, c):
    return 1 if (b if a else c) else 2


def h6(a, b, c):
    if a:
        while (b if a else c):
            g()
"#,
        expected: &[
            ("def h1", "h1", 2),
            ("def h2", "h2", 2),
            ("def h3", "h3", 2),
            ("def h4", "h4", 2),
            ("def h5", "h5", 2),
            ("def h6", "h6", 5),
        ],
    },
];
