use super::{ANY_NAME, Case};

pub const CASES: &[Case] = &[Case {
    name: "init deinit getter setter and method",
    file: "a.swift",
    source: r#"class A {
    var x = true
    init(_ a: Bool) {
        if a {
            if a {
                g()
            }
        }
    }
    deinit {
        if x {
            if x {
                g()
            }
        }
    }
    var v: Int {
        get {
            if x {
                if x {
                    return 1
                }
            }
            return 0
        }
        set {
            if x {
                if x {
                    g()
                }
            }
        }
    }
    func m(_ a: Bool) {
        if a {
            if a {
                g()
            }
        }
    }
}
"#,
    expected: &[
        ("    init(", ANY_NAME, 3),
        ("    deinit", ANY_NAME, 3),
        ("        get {", ANY_NAME, 3),
        ("        set {", ANY_NAME, 3),
        ("    func m", "m", 3),
    ],
}];
