use super::Case;

pub const CASES: &[Case] = &[
    Case {
        name: "nesting is kept after a plain else inside a chained if",
        file: "a.ts",
        source: r#"function f(xs, a, b, c) {
  for (const x of xs) {
    if (a) {
      if (b) {
        g();
      } else {
        h();
      }
    } else if (c) {
      g();
    }
  }
}
"#,
        expected: &[("function f", "f", 8)],
    },
    Case {
        name: "nesting is kept after plain elses in the exportState shape",
        file: "a.ts",
        source: r#"function f(x, y, z) {
  if (x) {
    if (y) {
      g();
    } else {
      for (;;) {
        g();
      }
    }
  } else {
    if (z) {
      g();
    } else {
      h();
    }
  }
}
"#,
        expected: &[("function f", "f", 11)],
    },
    Case {
        name: "nullish coalescing is not a boolean operator",
        file: "a.ts",
        source: "function f(a, b) {\n  if (a) {\n    g();\n  }\n  if (b) {\n    g();\n  }\n  return a ?? \"y\";\n}\n",
        expected: &[("function f", "f", 2)],
    },
    Case {
        name: "nullish chain beside an && run counts only the && run",
        file: "a.ts",
        source: "function f(a, b, c, d) {\n  if (a) {\n    g();\n  }\n  return a ?? b && c || d;\n}\n",
        expected: &[("function f", "f", 2)],
    },
    Case {
        name: "|| is not a boolean operator",
        file: "a.ts",
        source: "function f(a, b) {\n  if (a || b) {\n    g();\n  }\n  return a && b;\n}\n",
        expected: &[("function f", "f", 2)],
    },
    Case {
        name: "else if adds one and keeps the nesting after a nested plain else",
        file: "a.ts",
        source: "function f(xs, a, b, c) {\n  for (const x of xs) {\n    if (a) {\n      if (b) {\n        g();\n      } else {\n        h();\n      }\n    } else if (c) {\n      g();\n    }\n  }\n}\n",
        expected: &[("function f", "f", 8)],
    },
    Case {
        name: "plain else leak shape: else then a following if",
        file: "a.ts",
        source: "function f(a, b, c) {\n  if (a) {\n    if (b) {\n    } else {\n    }\n    if (c) {\n    }\n  }\n}\n",
        expected: &[("function f", "f", 6)],
    },
    Case {
        name: "without the else the same shape scores five",
        file: "a.ts",
        source: "function f(a, b, c) {\n  if (a) {\n    if (b) {\n    }\n    if (c) {\n    }\n  }\n}\n",
        expected: &[("function f", "f", 5)],
    },
];
