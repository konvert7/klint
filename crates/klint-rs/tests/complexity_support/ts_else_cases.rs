use super::Case;

pub const CASES: &[Case] = &[
    Case {
        name: "nesting is kept after a plain else inside a chained if (Biome scores 8)",
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
        expected: &[("function f", "f", 9)],
    },
    Case {
        name: "nesting is kept after plain elses in the exportState shape (Biome scores 10)",
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
        name: "nullish coalescing counts like a boolean operator",
        file: "a.ts",
        source: "function f(a, b, c, d) {\n  return a ?? b && c || d;\n}\n",
        expected: &[("function f", "f", 3)],
    },
];
