use super::Case;

pub const CASES: &[Case] = &[
    Case {
        name: "nested if",
        file: "a.ts",
        source: r#"function f(a, b) {
  if (a) {
    if (b) {
      g();
    }
  }
}
"#,
        expected: &[("function f", "f", 3)],
    },
    Case {
        name: "else if and else",
        file: "a.ts",
        source: r#"function f(a, b) {
  if (a) {
    g();
  } else if (b) {
    g();
  } else {
    g();
  }
}
"#,
        expected: &[("function f", "f", 3)],
    },
    Case {
        name: "else if inside for gets the nesting increment",
        file: "a.ts",
        source: r#"function f(xs, a, b) {
  for (const x of xs) {
    if (a) {
      g();
    } else if (b) {
      g();
    }
  }
}
"#,
        expected: &[("function f", "f", 5)],
    },
    Case {
        name: "ternary",
        file: "a.ts",
        source: r#"function f(a) {
  if (a) {
    g();
  }
  return a ? 1 : 2;
}
"#,
        expected: &[("function f", "f", 2)],
    },
    Case {
        name: "ternary inside if is nested",
        file: "a.ts",
        source: r#"function f(a) {
  if (a) {
    return a ? 1 : 2;
  }
}
"#,
        expected: &[("function f", "f", 3)],
    },
    Case {
        name: "switch",
        file: "a.ts",
        source: r#"function f(a) {
  switch (a) {
    case 1:
      g();
      break;
    default:
      h();
  }
  if (a) {
    g();
  }
}
"#,
        expected: &[("function f", "f", 2)],
    },
    Case {
        name: "every loop kind",
        file: "a.ts",
        source: r#"function f(xs, a) {
  for (let i = 0; i < 3; i++) {
    g();
  }
  for (const k in xs) {
    g();
  }
  for (const x of xs) {
    g();
  }
  while (a) {
    g();
  }
  do {
    g();
  } while (a);
}
"#,
        expected: &[("function f", "f", 5)],
    },
    Case {
        name: "catch and finally",
        file: "a.ts",
        source: r#"function f(a) {
  try {
    g();
  } catch (e) {
    if (a) {
      g();
    }
  } finally {
    if (a) {
      g();
    }
  }
}
"#,
        expected: &[("function f", "f", 5)],
    },
    Case {
        name: "labeled break",
        file: "a.ts",
        source: r#"function f(xs, ys) {
  outer: for (const x of xs) {
    for (const y of ys) {
      if (y) {
        break outer;
      }
    }
  }
}
"#,
        expected: &[("function f", "f", 7)],
    },
    Case {
        name: "labeled continue",
        file: "a.ts",
        source: r#"function f(xs, a) {
  outer: for (const x of xs) {
    if (a) {
      continue outer;
    }
  }
}
"#,
        expected: &[("function f", "f", 4)],
    },
    Case {
        name: "unlabeled break and continue add nothing",
        file: "a.ts",
        source: r#"function f(xs, a) {
  for (const x of xs) {
    if (a) {
      break;
    }
    if (x) {
      continue;
    }
  }
}
"#,
        expected: &[("function f", "f", 5)],
    },
    Case {
        name: "runs of boolean operators",
        file: "a.ts",
        source: r#"function r1(a, b, c, d) {
  return a && b && c || d;
}
function r2(a, b, c) {
  return a && (b || c);
}
function r3(a, b, c) {
  return a && (b || c) && a;
}
function r6(a, b, c) {
  return a && (b && c);
}
function r4(a, b, c) {
  if (a) {
    g();
  }
  return a ?? b ?? c;
}
function r5(a, b, c) {
  return (a ?? b) && c;
}
"#,
        expected: &[
            ("function r1", "r1", 2),
            ("function r2", "r2", 2),
            ("function r3", "r3", 2),
            ("function r4", "r4", 2),
            ("function r5", "r5", 2),
            ("function r6", "r6", 2),
        ],
    },
    Case {
        name: "negation and assignment operators add nothing",
        file: "a.ts",
        source: r#"function n1(a, b) {
  if (!a && !b) {
    g();
  }
}
function n2(a, b) {
  if (a) {
    g();
  }
  if (b) {
    g();
  }
  a ||= b;
  a &&= b;
  a ??= b;
}
"#,
        expected: &[("function n1", "n1", 2), ("function n2", "n2", 2)],
    },
    Case {
        name: "try, return, throw, recursion and optional chaining add nothing",
        file: "a.ts",
        source: r#"function f(a) {
  if (a?.b) {
    return f(a.c);
  }
  if (a) {
    throw a;
  }
  try {
    g();
  } finally {
  }
}
"#,
        expected: &[("function f", "f", 3)],
    },
    Case {
        name: "finally adds one flat and its body is not nested",
        file: "a.ts",
        source: r#"function e1(a) {
  if (a) {
    g();
  }
  try {
    g();
  } finally {
  }
}
function e2(a, b) {
  try {
    g();
  } finally {
    if (a) {
      if (b) {
      }
    }
  }
}
"#,
        expected: &[("function e1", "e1", 2), ("function e2", "e2", 4)],
    },
    Case {
        name: "arrow directly in a body starts at nesting one",
        file: "a.ts",
        source: r#"function outer(a) {
  if (a) {
    g();
  }
  if (a) {
    g();
  }
  const inner = (x) => {
    if (x) {
      g();
    }
    return x ? 1 : 2;
  };
  return inner;
}
"#,
        expected: &[("function outer", "outer", 2), ("const inner", "inner", 4)],
    },
    Case {
        name: "closure inside an if inherits the nesting and does not add to the outer",
        file: "a.ts",
        source: r#"function outer(a, b, c) {
  if (a) {
    const inner = () => {
      if (b) {
        if (c) {
          g();
        }
      }
    };
    inner();
  }
  if (b) {
    g();
  }
}
"#,
        expected: &[("function outer", "outer", 2), ("const inner", "inner", 7)],
    },
    Case {
        name: "function nested in a nested function keeps accumulating",
        file: "a.ts",
        source: r#"function a1(a, b) {
  if (a) {
    g();
  }
  if (b) {
    g();
  }
  const b1 = () => {
    if (a) {
      const c1 = () => {
        if (b) {
          g();
        }
      };
      c1();
    }
  };
  b1();
}
"#,
        expected: &[
            ("function a1", "a1", 2),
            ("const b1", "b1", 2),
            ("const c1", "c1", 4),
        ],
    },
    Case {
        name: "class declared in a function adds no nesting and its methods start one deeper",
        file: "a.ts",
        source: r#"function f(a) {
  if (a) {
    class C {
      m(b) {
        if (b) {
          g();
        }
      }
    }
  }
  if (a) {
    g();
  }
}
"#,
        expected: &[("function f", "f", 2), ("      m(b)", "m", 3)],
    },
    Case {
        name: "boolean operators are counted per function across a nested boundary",
        file: "a.ts",
        source: r#"function f(a, b, c) {
  if (a) {
    g();
  }
  return a && b && (() => {
    if (c) {
      g();
    }
    return c || a || b;
  })();
}
"#,
        expected: &[("function f", "f", 2), ("(() => {", "<anonymous>", 3)],
    },
    Case {
        name: "methods getters setters and generators",
        file: "a.ts",
        source: r#"class A {
  m(a) {
    if (a) {
      if (a) {
        g();
      }
    }
  }
  get v() {
    if (this.a) {
      if (this.a) {
        return 1;
      }
    }
    return 0;
  }
  set v(x) {
    if (x) {
      if (x) {
        g();
      }
    }
  }
}
function* gen(a) {
  if (a) {
    if (a) {
      yield 1;
    }
  }
}
"#,
        expected: &[
            ("  m(a)", "m", 3),
            ("  get v()", "v", 3),
            ("  set v(x)", "v", 3),
            ("function* gen", "gen", 3),
        ],
    },
    Case {
        name: "empty bodies and signatures are not functions",
        file: "a.ts",
        source: r#"function empty() {}
const e = () => {};
interface I {
  m(a: number): void;
}
abstract class B {
  abstract n(a: number): void;
}
function real(a) {
  if (a) {
    g();
  }
  if (a) {
    g();
  }
}
"#,
        expected: &[("function real", "real", 2)],
    },
    Case {
        name: "two functions on the same line are both reported in column order",
        file: "a.ts",
        source: r#"const a = (x) => { if (x) { if (x) { g(); } } }; const b = (y) => { if (y) { if (y) { if (y) { g(); } } } };
"#,
        expected: &[("const a", "a", 3), ("const a", "b", 6)],
    },
    Case {
        name: "top-level code is not scored",
        file: "a.ts",
        source: r#"const a = true;
if (a) {
  if (a) {
    g();
  }
}
function real(a) {
  if (a) {
    g();
  }
  if (a) {
    g();
  }
}
"#,
        expected: &[("function real", "real", 2)],
    },
];
