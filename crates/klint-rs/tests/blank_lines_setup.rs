mod blank_lines_support;
mod common;

use blank_lines_support::*;

type Case = (&'static str, &'static str, &'static [u64]);

const TS: &[Case] = &[
    ("assigned name used", "x = f();\nif (x) {\n  g();\n}\n", &[]),
    (
        "declared name used by switch",
        "const y = f();\nswitch (y) {\n  case 1:\n    g();\n}\n",
        &[],
    ),
    ("name not used", "x = f();\nif (z) {\n  g();\n}\n", &[2]),
    (
        "for is never exempt",
        "x = f();\nfor (const i of x) {\n  g();\n}\n",
        &[2],
    ),
    (
        "while is never exempt",
        "x = f();\nwhile (x) {\n  g();\n}\n",
        &[2],
    ),
    ("return is never exempt", "x = f();\nreturn x;\n", &[2]),
    (
        "member target assigns no name",
        "self.x = f();\nif (self.x) {\n  g();\n}\n",
        &[2],
    ),
    ("compound assignment", "x += 1;\nif (x) {\n  g();\n}\n", &[]),
    ("whole names only", "xy = f();\nif (x) {\n  g();\n}\n", &[2]),
    (
        "body is not searched",
        "x = f();\nif (z) {\n  x;\n}\n",
        &[2],
    ),
    (
        "destructuring",
        "const { a, b } = f();\nif (b) {\n  g();\n}\n",
        &[],
    ),
    (
        "multi-line setup",
        "const y = f(\n  1,\n);\nif (y) {\n  g();\n}\n",
        &[],
    ),
    (
        "only the statement directly above counts",
        "x = 1;\ny = 2;\nif (x) {\n  g();\n}\n",
        &[3],
    ),
];

const PY: &[Case] = &[
    ("assigned name used", "x = f()\nif x:\n    g()\n", &[]),
    ("name not used", "x = f()\nif z:\n    g()\n", &[2]),
    (
        "for is never exempt",
        "x = f()\nfor i in x:\n    g()\n",
        &[2],
    ),
    (
        "while is never exempt",
        "x = f()\nwhile x:\n    g()\n",
        &[2],
    ),
    ("return is never exempt", "x = f()\nreturn x\n", &[2]),
    ("member target", "self.x = f()\nif self.x:\n    g()\n", &[2]),
    (
        "match subject",
        "x = f()\nmatch x:\n    case 1:\n        g()\n",
        &[],
    ),
];

const SWIFT: &[Case] = &[
    (
        "declared name used",
        "let x = f()\nif x {\n    g()\n}\n",
        &[],
    ),
    (
        "guard condition",
        "let x = f()\nguard x else {\n    return\n}\n",
        &[],
    ),
    ("name not used", "let x = f()\nif y {\n    g()\n}\n", &[2]),
    (
        "for is never exempt",
        "let x = f()\nfor i in x {\n    g()\n}\n",
        &[2],
    ),
    (
        "member target",
        "self.x = f()\nif self.x {\n    g()\n}\n",
        &[2],
    ),
    (
        "switch subject",
        "let x = f()\nswitch x {\ncase 1:\n    g()\ndefault:\n    break\n}\n",
        &[],
    ),
    (
        "tuple pattern",
        "let (a, b) = f()\nif b {\n    g()\n}\n",
        &[],
    ),
];

const RS: &[Case] = &[
    (
        "declared name used",
        "fn m() {\n    let x = f();\n    if x {\n        g();\n    }\n}\n",
        &[],
    ),
    (
        "let else binds q",
        "fn m() {\n    let Some(q) = f() else {\n        return;\n    };\n    if q {\n        g();\n    }\n}\n",
        &[],
    ),
    (
        "constructor names are not assigned",
        "fn m() {\n    let Some(q) = f() else {\n        return;\n    };\n    if Some(1) == z {\n        g();\n    }\n}\n",
        &[5],
    ),
    (
        "compound assignment",
        "fn m() {\n    x += 1;\n    if x > 2 {\n        g();\n    }\n}\n",
        &[],
    ),
    (
        "member target",
        "fn m() {\n    self.x = f();\n    if self.x {\n        g();\n    }\n}\n",
        &[3],
    ),
    (
        "match subject",
        "fn m() {\n    let x = f();\n    match x {\n        _ => {}\n    }\n}\n",
        &[],
    ),
    (
        "for is never exempt",
        "fn m() {\n    let x = f();\n    for i in x {\n        g();\n    }\n}\n",
        &[3],
    ),
    (
        "return is never exempt",
        "fn m() {\n    let x = f();\n    return x;\n}\n",
        &[3],
    ),
];

const CS: &[Case] = &[
    (
        "declared name used",
        "var x = F();\nif (x) {\n    G();\n}\n",
        &[],
    ),
    (
        "switch subject",
        "x = F();\nswitch (x) {\n    case 1:\n        G();\n        break;\n}\n",
        &[],
    ),
    (
        "name not used",
        "var x = F();\nif (y) {\n    G();\n}\n",
        &[2],
    ),
    (
        "foreach is never exempt",
        "var x = F();\nforeach (var i in x) {\n    G();\n}\n",
        &[2],
    ),
    (
        "while is never exempt",
        "var x = F();\nwhile (x) {\n    G();\n}\n",
        &[2],
    ),
    (
        "member target",
        "this.x = F();\nif (this.x) {\n    G();\n}\n",
        &[2],
    ),
    ("return is never exempt", "var x = F();\nreturn x;\n", &[2]),
];

fn check(file: &str, cases: &[Case]) {
    for (name, source, expected) in cases {
        let fx = fixture("blank-setup", &simple(BEFORE), &[(file, source)]);
        let found = lines_of(&lint(&fx), BEFORE, file);
        assert_eq!(&found, expected, "{file}: {name}");
    }
}

#[test]
fn setup_exception_in_typescript() {
    check("a.ts", TS);
}

#[test]
fn setup_exception_in_python() {
    check("a.py", PY);
}

#[test]
fn setup_exception_in_swift() {
    check("a.swift", SWIFT);
}

#[test]
fn setup_exception_in_rust() {
    check("a.rs", RS);
}

#[test]
fn setup_exception_in_csharp() {
    check("a.cs", CS);
}

#[test]
fn allow_setup_false_turns_the_exception_off() {
    let config = config_for(&format!(
        "  {BEFORE}:\n    severity: error\n    allowSetup: false\n"
    ));
    for (file, source) in [
        ("a.ts", "x = f();\nif (x) {\n  g();\n}\n"),
        ("a.py", "x = f()\nif x:\n    g()\n"),
        ("a.swift", "let x = f()\nif x {\n    g()\n}\n"),
        ("a.cs", "var x = F();\nif (x) {\n    G();\n}\n"),
    ] {
        let fx = fixture("blank-setup-off", &config, &[(file, source)]);
        assert_eq!(lines_of(&lint(&fx), BEFORE, file), vec![2], "{file}");
    }
}
