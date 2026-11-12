export const COUNTING_ROWS = [
  "braces",
  "comment",
  "blank",
  "trailing-blank",
  "closing-brace-semicolon",
  "template",
  "string",
  "nested-arrow",
  "expression-arrow",
  "object-braces",
  "method",
  "constructor",
  "accessor",
  "generator",
  "function-expression",
  "block-comment",
  "jsx",
] as const;

interface CorpusEntry {
  row: (typeof COUNTING_ROWS)[number];
  code: string;
  offset?: number;
  accessor?: boolean;
}

export const CORPUS: CorpusEntry[] = [
  { row: "braces", code: "function c1() {\n  a();\n  b();\n}\n" },
  { row: "braces", code: "function c2() {\n  if (a) {\n    b();\n  }\n  c();\n}\n" },
  { row: "comment", code: "function c3() {\n  // c\n  a();\n}\n" },
  { row: "comment", code: "function c4() {\n  a(); // t\n  // c\n  b();\n}\n" },
  { row: "block-comment", code: "function c5() {\n  a();\n  /* x\n  y */\n  b();\n}\n" },
  { row: "blank", code: "function c6() {\n  a();\n\n  b();\n}\n" },
  { row: "blank", code: "function c7() {\n  a();\n\n\n\n  b();\n}\n" },
  { row: "trailing-blank", code: "function c8() {\n  x(); // t\n  y();\n\n\n}\n" },
  { row: "trailing-blank", code: "function c9() {\n  x();\n  y();\n\n}\n" },
  {
    row: "closing-brace-semicolon",
    code: "function c10() {\n  const o = {\n    a: 1,\n  };\n  b();\n}\n",
  },
  {
    row: "template",
    code: "function c11() {\n  const s = `a\nb\nc`;\n  g();\n}\n",
  },
  {
    row: "template",
    code: "function c12() {\n  const s = `a$" + "{x}\nb$" + "{y}\nc`;\n  g();\n}\n",
  },
  { row: "string", code: 'function c13() {\n  const s = "a\\\nb";\n  g();\n}\n' },
  {
    row: "nested-arrow",
    code: "function c14() {\n  const inner = () => {\n    a();\n    b();\n  };\n}\n",
  },
  {
    row: "nested-arrow",
    code: "const c15 = () => {\n  const inner = () => {\n    const deep = () => {\n      a();\n      b();\n    };\n  };\n};\n",
  },
  { row: "expression-arrow", code: "const c16 = (x) =>\n  foo(\n    x,\n  );\n" },
  { row: "expression-arrow", code: "function c17() {\n  return (x) =>\n    x + 1;\n}\n" },
  {
    row: "object-braces",
    code: "function c18() {\n  return {\n    a: 1,\n    b: 2,\n  };\n}\n",
  },
  {
    row: "object-braces",
    code: "function c19() {\n  const { a, b } = x;\n  const o = { a, b };\n  g(o);\n}\n",
  },
  {
    row: "method",
    code: "class C20 {\n  m() {\n    a();\n    b();\n  }\n}\n",
    offset: 1,
  },
  {
    row: "constructor",
    code: "class C21 {\n  constructor() {\n    a();\n    b();\n  }\n}\n",
    offset: 1,
  },
  {
    row: "method",
    code: "const o22 = {\n  m() {\n    a();\n    b();\n  },\n};\n",
    offset: 1,
  },
  {
    row: "accessor",
    code: "class C23 {\n  get g() {\n    a();\n    b();\n  }\n}\n",
    offset: 1,
    accessor: true,
  },
  {
    row: "accessor",
    code: "class C24 {\n  set s(v) {\n    a();\n    b();\n  }\n}\n",
    offset: 1,
    accessor: true,
  },
  { row: "generator", code: "function* c25() {\n  yield 1;\n  yield 2;\n}\n" },
  {
    row: "function-expression",
    code: "const c26 = function () {\n  a();\n  b();\n};\n",
  },
  {
    row: "function-expression",
    code: "foo(function () {\n  a();\n  b();\n});\n",
  },
  {
    row: "jsx",
    code: "function c28() {\n  return (\n    <div>\n      {x}\n    </div>\n  );\n}\n",
  },
  {
    row: "braces",
    code: "function c29() {\n  try {\n    a();\n  } catch (e) {\n    b();\n  } finally {\n    c();\n  }\n}\n",
  },
  {
    row: "blank",
    code: "function c30() {\n\n  a();\n\n  // c\n\n  b();\n}\n",
  },
];
