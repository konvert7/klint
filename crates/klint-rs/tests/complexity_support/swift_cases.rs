use super::Case;

pub const CASES: &[Case] = &[
    Case {
        name: "if else if else",
        file: "a.swift",
        source: r#"func f(_ a: Bool, _ b: Bool) {
    if a {
        g()
    } else if b {
        g()
    } else {
        g()
    }
}
"#,
        expected: &[("func f", "f", 3)],
    },
    Case {
        name: "else if inside for gets the nesting increment",
        file: "a.swift",
        source: r#"func f(_ xs: [Int], _ a: Bool, _ b: Bool) {
    for x in xs {
        if a {
            g(x)
        } else if b {
            g(x)
        }
    }
}
"#,
        expected: &[("func f", "f", 5)],
    },
    Case {
        name: "nested if",
        file: "a.swift",
        source: r#"func f(_ a: Bool, _ b: Bool) {
    if a {
        if b {
            g()
        }
    }
}
"#,
        expected: &[("func f", "f", 3)],
    },
    Case {
        name: "ternary",
        file: "a.swift",
        source: r#"func f(_ a: Bool) -> Int {
    if a {
        g()
    }
    return a ? 1 : 2
}
"#,
        expected: &[("func f", "f", 2)],
    },
    Case {
        name: "switch",
        file: "a.swift",
        source: r#"func f(_ a: Int) {
    switch a {
    case 1:
        g()
    default:
        h()
    }
    if a > 0 {
        g()
    }
}
"#,
        expected: &[("func f", "f", 2)],
    },
    Case {
        name: "case where",
        file: "a.swift",
        source: r#"func f(_ a: Int) {
    switch a {
    case let x where x > 1:
        g()
    default:
        h()
    }
}
"#,
        expected: &[("func f", "f", 2)],
    },
    Case {
        name: "guard counts like if and nests its else body",
        file: "a.swift",
        source: r#"func g1(_ a: Int?) {
    guard let x = a else {
        return
    }
    if x > 1 {
        g()
    }
}
func g2(_ a: Bool, _ b: Bool) {
    guard a else {
        if b {
            g()
        }
        return
    }
}
"#,
        expected: &[("func g1", "g1", 2), ("func g2", "g2", 3)],
    },
    Case {
        name: "for while and repeat while",
        file: "a.swift",
        source: r#"func f(_ xs: [Int], _ a: Bool) {
    for x in xs {
        g(x)
    }
    while a {
        g(0)
    }
    repeat {
        g(0)
    } while a
}
"#,
        expected: &[("func f", "f", 3)],
    },
    Case {
        name: "catch counts and a do without catch adds nothing",
        file: "a.swift",
        source: r#"func c1(_ a: Bool) {
    do {
        try g()
    } catch {
        if a {
            h()
        }
    }
}
func c2(_ a: Bool) {
    do {
        g()
    }
    if a {
        g()
    }
    if a {
        g()
    }
}
"#,
        expected: &[("func c1", "c1", 3), ("func c2", "c2", 2)],
    },
    Case {
        name: "runs of boolean operators and nil coalescing",
        file: "a.swift",
        source: r#"func r1(_ a: Bool, _ b: Bool, _ c: Bool) -> Bool {
    return a && b || c
}
func r2(_ a: Bool, _ b: Bool, _ c: Bool) -> Bool {
    return a || b && c
}
func r3(_ a: Bool, _ b: Bool, _ c: Bool) -> Bool {
    return a && (b || c) && a
}
func r4(_ a: Int?, _ b: Int?, _ c: Int) -> Int {
    if a == nil {
        g()
    }
    return a ?? b ?? c
}
func r5(_ a: Bool, _ b: Bool) {
    if !a && !b {
        g()
    }
}
func r6(_ a: Bool, _ b: Bool, _ c: Bool, _ d: Bool) -> Bool {
    return a && b && c || d
}
"#,
        expected: &[
            ("func r1", "r1", 2),
            ("func r2", "r2", 2),
            ("func r3", "r3", 3),
            ("func r4", "r4", 2),
            ("func r5", "r5", 2),
            ("func r6", "r6", 2),
        ],
    },
    Case {
        name: "labeled continue and break",
        file: "a.swift",
        source: r#"func l1(_ xs: [Int], _ ys: [Int]) {
    outer: for x in xs {
        for y in ys {
            if y > x {
                continue outer
            }
        }
    }
}
func l2(_ xs: [Int], _ ys: [Int]) {
    outer: for x in xs {
        for y in ys {
            if y > x {
                break outer
            }
        }
    }
}
func l3(_ xs: [Int]) {
    for x in xs {
        if x > 0 {
            continue
        }
        if x < 0 {
            break
        }
    }
}
"#,
        expected: &[
            ("func l1", "l1", 7),
            ("func l2", "l2", 7),
            ("func l3", "l3", 5),
        ],
    },
    Case {
        name: "closure inside an if inherits the nesting",
        file: "a.swift",
        source: r#"func f(_ a: Bool, _ b: Bool, _ c: Bool) {
    if a {
        let g = {
            if b {
                if c {
                    h()
                }
            }
        }
        g()
    }
    if b {
        h()
    }
}
"#,
        expected: &[("func f", "f", 2), ("let g = {", "g", 7)],
    },
    Case {
        name: "anonymous trailing closure",
        file: "a.swift",
        source: r#"func f(_ xs: [Int], _ a: Bool) {
    if a {
        g()
    }
    if a {
        g()
    }
    xs.forEach { x in
        if x > 0 {
            g()
        }
    }
}
"#,
        expected: &[("func f", "f", 2), ("xs.forEach", "<anonymous>", 2)],
    },
    Case {
        name: "empty function and protocol requirement are skipped",
        file: "a.swift",
        source: r#"func empty() {}

protocol P {
    func m(_ a: Int)
}

func real(_ a: Bool) {
    if a {
        g()
    }
    if a {
        g()
    }
}
"#,
        expected: &[("func real", "real", 2)],
    },
    Case {
        name: "two functions on the same line are both reported in column order",
        file: "a.swift",
        source: r#"func a(_ x: Bool) { if x { if x { g() } } }; func b(_ y: Bool) { if y { if y { if y { g() } } } }
"#,
        expected: &[("func a", "a", 3), ("func a", "b", 6)],
    },
    Case {
        name: "top-level code is not scored",
        file: "main.swift",
        source: r#"let a = true
if a {
    if a {
        g()
    }
}
func real(_ a: Bool) {
    if a {
        g()
    }
    if a {
        g()
    }
}
"#,
        expected: &[("func real", "real", 2)],
    },
];
