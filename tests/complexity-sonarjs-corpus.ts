export const SCORING_ROWS = [
  "if",
  "else-if",
  "else",
  "ternary",
  "switch",
  "loop",
  "catch",
  "finally",
  "labeled-jump",
  "boolean-run",
  "nested-function",
  "class-in-function",
  "no-increment",
] as const;

interface CorpusEntry {
  row: (typeof SCORING_ROWS)[number];
  code: string;
  offset?: number;
  expected?: number;
}

export const CORPUS: CorpusEntry[] = [
  {
    row: "if",
    code: "function e1(a, b) {\n  if (a) {\n    if (b) {\n      g();\n    }\n  }\n}\n",
  },
  {
    row: "else-if",
    code: "function e2(a, b) {\n  if (a) {\n    g();\n  } else if (b) {\n    g();\n  } else {\n    g();\n  }\n}\n",
  },
  {
    row: "else-if",
    code: "function e3(xs, a, b) {\n  for (const x of xs) {\n    if (a) {\n      g();\n    } else if (b) {\n      g();\n    }\n  }\n}\n",
  },
  {
    row: "else",
    code: "function e4(a) {\n  if (a) {\n    g();\n  } else {\n    h();\n  }\n}\n",
  },
  {
    row: "ternary",
    code: "function e5(a) {\n  if (a) {\n    g();\n  }\n  return a ? 1 : 2;\n}\n",
  },
  {
    row: "ternary",
    code: "function e6(a) {\n  if (a) {\n    return a ? 1 : 2;\n  }\n}\n",
  },
  {
    row: "ternary",
    code: "function e7(a, b) {\n  return a ? (b ? 1 : 2) : 3;\n}\n",
  },
  {
    row: "switch",
    code: "function e8(a) {\n  switch (a) {\n    case 1:\n      g();\n      break;\n    default:\n      h();\n  }\n  if (a) {\n    g();\n  }\n}\n",
  },
  {
    row: "switch",
    code: "function e9(xs, a) {\n  for (const x of xs) {\n    switch (x) {\n      case 1:\n        if (a) {\n          g();\n        }\n        break;\n    }\n  }\n}\n",
  },
  {
    row: "loop",
    code: "function e10(xs, a) {\n  for (let i = 0; i < 3; i++) {\n    g();\n  }\n  for (const k in xs) {\n    g();\n  }\n  for (const x of xs) {\n    g();\n  }\n  while (a) {\n    g();\n  }\n  do {\n    g();\n  } while (a);\n}\n",
  },
  {
    row: "loop",
    code: "function e11(a, b) {\n  while (a && b) {\n    if (a) {\n      g();\n    } else {\n      h();\n    }\n  }\n}\n",
  },
  {
    row: "loop",
    code: "function e12(a) {\n  do {\n    if (a) {\n      g();\n    }\n  } while (a);\n}\n",
  },
  {
    row: "catch",
    code: "function e13(a) {\n  try {\n    g();\n  } catch (e) {\n    if (a) {\n      g();\n    }\n  }\n}\n",
  },
  {
    row: "finally",
    code: "function e14(a) {\n  try {\n    g();\n  } finally {\n    if (a) {\n      g();\n    }\n  }\n}\n",
  },
  {
    row: "catch",
    code: "function e15(a) {\n  if (a) {\n    try {\n      g();\n    } catch (e) {\n      if (e) {\n        h();\n      }\n    }\n  }\n}\n",
  },
  {
    row: "labeled-jump",
    code: "function e16(xs, ys) {\n  outer: for (const x of xs) {\n    for (const y of ys) {\n      if (y) {\n        break outer;\n      }\n    }\n  }\n}\n",
  },
  {
    row: "labeled-jump",
    code: "function e17(xs, a) {\n  outer: for (const x of xs) {\n    if (a) {\n      continue outer;\n    }\n  }\n}\n",
  },
  {
    row: "labeled-jump",
    code: "function e18(xs, a) {\n  for (const x of xs) {\n    if (a) {\n      break;\n    }\n    if (x) {\n      continue;\n    }\n  }\n}\n",
  },
  {
    row: "boolean-run",
    code: "function e19(a, b, c, d) {\n  return a && b && c || d;\n}\n",
  },
  {
    row: "boolean-run",
    code: "function e20(a, b, c) {\n  return a && (b || c);\n}\n",
  },
  {
    row: "boolean-run",
    code: "function e21(a, b, c) {\n  return a && (b || c) && a;\n}\n",
  },
  {
    row: "boolean-run",
    code: "function e22(a, b, c) {\n  if (a) {\n    g();\n  }\n  return a ?? b ?? c;\n}\n",
  },
  {
    row: "boolean-run",
    code: "function e23(a, b, c) {\n  return (a ?? b) && c;\n}\n",
  },
  {
    row: "no-increment",
    code: "function e24(a, b) {\n  if (!a && !b) {\n    g();\n  }\n}\n",
  },
  {
    row: "no-increment",
    code: "function e25(a, b) {\n  if (a) {\n    g();\n  }\n  if (b) {\n    g();\n  }\n  a ||= b;\n  a &&= b;\n  a ??= b;\n}\n",
  },
  {
    row: "no-increment",
    code: "function e26(a) {\n  if (a?.b) {\n    return e26(a.c);\n  }\n  if (a) {\n    throw a;\n  }\n}\n",
  },
  {
    row: "nested-function",
    code: "function e27(a) {\n  if (a) {\n    g();\n  }\n  if (a) {\n    g();\n  }\n  const inner = (x) => {\n    if (x) {\n      g();\n    }\n    return x ? 1 : 2;\n  };\n  return inner;\n}\n",
  },
  {
    row: "nested-function",
    code: "function e28(a, b, c) {\n  if (a) {\n    const inner = () => {\n      if (b) {\n        if (c) {\n          g();\n        }\n      }\n    };\n    inner();\n  }\n  if (b) {\n    g();\n  }\n}\n",
  },
  {
    row: "nested-function",
    code: "function e29(a, b) {\n  if (a) {\n    g();\n  }\n  if (b) {\n    g();\n  }\n  const b1 = function () {\n    if (a) {\n      const c1 = () => {\n        if (b) {\n          g();\n        }\n      };\n      c1();\n    }\n  };\n  b1();\n}\n",
  },
  {
    row: "nested-function",
    code: "const e30 = (a) => {\n  if (a) {\n    if (a) {\n      g();\n    }\n  }\n};\n",
  },
  {
    row: "nested-function",
    code: "function e31(a, b, c) {\n  if (a) {\n    g();\n  }\n  return a && b && (() => {\n    if (c) {\n      g();\n    }\n    return c || a || b;\n  })();\n}\n",
  },
  {
    row: "class-in-function",
    code: "function e32(a) {\n  if (a) {\n    class C {\n      m(b) {\n        if (b) {\n          g();\n        }\n      }\n    }\n  }\n  if (a) {\n    g();\n  }\n}\n",
  },
  {
    row: "class-in-function",
    offset: 1,
    code: "class E33 {\n  m(a) {\n    if (a) {\n      if (a) {\n        g();\n      }\n    }\n  }\n}\n",
  },
  {
    row: "nested-function",
    code: "function* e34(a) {\n  if (a) {\n    if (a) {\n      yield 1;\n    }\n  }\n}\n",
  },
  {
    row: "nested-function",
    offset: 1,
    code: "const e35 = {\n  m(a) {\n    if (a) {\n      if (a) {\n        g();\n      }\n    }\n  },\n};\n",
  },
  {
    row: "finally",
    code: "function e36(a) {\n  if (a) {\n    g();\n  }\n  try {\n    g();\n  } finally {\n  }\n}\n",
  },
  {
    row: "finally",
    code: "function e37(a, b) {\n  try {\n    g();\n  } finally {\n    if (a) {\n      if (b) {\n      }\n    }\n  }\n}\n",
  },
  {
    row: "boolean-run",
    code: "function e38(a, b, c) {\n  return a && (b && c);\n}\n",
  },
  {
    row: "else",
    code: "function e39(xs, a, b, c) {\n  for (const x of xs) {\n    if (a) {\n      if (b) {\n        g();\n      } else {\n        h();\n      }\n    } else if (c) {\n      g();\n    }\n  }\n}\n",
  },
  {
    row: "else",
    code: "function e40(x, y, z) {\n  if (x) {\n    if (y) {\n      g();\n    } else {\n      for (;;) {\n        g();\n      }\n    }\n  } else {\n    if (z) {\n      g();\n    } else {\n      h();\n    }\n  }\n}\n",
  },
  {
    row: "boolean-run",
    code: "function e41(a, b) {\n  if (a) {\n    g();\n  }\n  return a ?? b;\n}\n",
  },
  {
    row: "boolean-run",
    code: "function e42(a, b, c, d) {\n  return a ?? b && c || d;\n}\n",
  },
  {
    row: "boolean-run",
    code: "function e43(a, b, c) {\n  return a ? b ?? c : c;\n}\n",
  },
  {
    row: "boolean-run",
    expected: 2,
    code: 'function s1(a, b) {\n  if (a) {\n    g();\n  }\n  if (b) {\n    g();\n  }\n  return a ?? "y";\n}\n',
  },
  {
    row: "boolean-run",
    expected: 2,
    code: "function s2(a, b) {\n  if (a || b) {\n  }\n  return a && b;\n}\n",
  },
  {
    row: "else",
    expected: 6,
    code: "function s3(a, b, c) {\n  if (a) {\n    if (b) {\n    } else {\n    }\n    if (c) {\n    }\n  }\n}\n",
  },
  {
    row: "else",
    expected: 5,
    code: "function s4(a, b, c) {\n  if (a) {\n    if (b) {\n    }\n    if (c) {\n    }\n  }\n}\n",
  },
  {
    row: "else-if",
    expected: 4,
    code: "function s5(xs, a, c) {\n  for (const x of xs) {\n    if (a) {\n    } else if (c) {\n    }\n  }\n}\n",
  },
  {
    row: "else-if",
    expected: 8,
    code: "function s6(xs, a, b, c) {\n  for (const x of xs) {\n    if (a) {\n      if (b) {\n      } else {\n      }\n    } else if (c) {\n    }\n  }\n}\n",
  },
  {
    row: "if",
    code: "function h1(a, b, c) {\n  if (a) {\n  }\n  if (a) {\n  }\n  if (a ? b : c) {\n  }\n}\n",
  },
  {
    row: "loop",
    code: "function h2(a, b, c) {\n  if (a) {\n  }\n  if (a) {\n  }\n  while (a ? b : c) {\n  }\n}\n",
  },
  {
    row: "loop",
    code: "function h3(a, b) {\n  if (a) {\n  }\n  if (a) {\n  }\n  for (let i = a ? 0 : 1; i < b; i++) {\n  }\n}\n",
  },
  {
    row: "switch",
    code: "function h4(a, b, c) {\n  if (a) {\n  }\n  if (a) {\n  }\n  switch (a ? b : c) {\n    case 1:\n      g();\n      break;\n  }\n}\n",
  },
  {
    row: "ternary",
    code: "function h5(a, b, c) {\n  if (a) {\n  }\n  if (a) {\n  }\n  return (a ? b : c) ? 1 : 2;\n}\n",
  },
  {
    row: "ternary",
    code: "function h6(a, b, c) {\n  if (a) {\n    if (b ? c : a) {\n    }\n  }\n}\n",
  },
  {
    row: "nested-function",
    code: "function t1(a, b) {\n  if (a) {\n  }\n  if (a) {\n  }\n  return a ? () => {\n    if (b) {\n      if (b) {\n      }\n    }\n  } : b;\n}\n",
  },
  {
    row: "nested-function",
    code: "function t2(a, b) {\n  if (a) {\n  }\n  if (a) {\n  }\n  return a ? b : () => {\n    if (b) {\n      if (b) {\n      }\n    }\n  };\n}\n",
  },
  {
    row: "nested-function",
    code: "function t3(a, b) {\n  if (a) {\n  }\n  if (a) {\n  }\n  return a ? function () {\n    if (b) {\n      if (b) {\n      }\n    }\n  } : b;\n}\n",
  },
  {
    row: "nested-function",
    code: "function t4(a, b) {\n  const r = a ? () => {\n    if (b) {\n      if (b) {\n      }\n    }\n  } : b;\n  if (a) {\n    if (b) {\n    }\n  }\n  return r;\n}\n",
  },
  {
    row: "nested-function",
    code: "function n1(a, b, c, d, e, f) {\n  if (a) {\n  }\n  if (a) {\n  }\n  return a ? () => {\n    const r = b ? () => {\n      if (c) {\n        if (c) {\n        }\n      }\n    } : d;\n    if (e) {\n    }\n    return r;\n  } : f;\n}\n",
  },
];
