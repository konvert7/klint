use super::EdgeCase;

pub const CASES: [EdgeCase; 13] = [
    EdgeCase {
        name: "ts_block_comment",
        file: "block_comment.ts",
        input: "function f() {
  a();
  /*
   * explains the loop
   */
  for (const y of z) {
    b();
  }
  /** jsdoc */
  return 1;
}
",
        expected: Some(
            "function f() {
  a();

  /*
   * explains the loop
   */
  for (const y of z) {
    b();
  }

  /** jsdoc */
  return 1;
}
",
        ),
    },
    EdgeCase {
        name: "ts_comment_before_else",
        file: "comment_before_else.ts",
        input: "function f() {
  if (x) {
    a();
  }
  // otherwise
  else {
    b();
  }
  try {
    c();
  }
  // handle
  catch (e) {
    d();
  }
}
",
        expected: Some(
            "function f() {
  if (x) {
    a();
  }
  // otherwise
  else {
    b();
  }

  try {
    c();
  }
  // handle
  catch (e) {
    d();
  }
}
",
        ),
    },
    EdgeCase {
        name: "ts_comment_first_in_block",
        file: "comment_first_in_block.ts",
        input: "function f() {
  // leading
  if (x) {
    // inside, first
    return 1;
  }
}
",
        expected: None,
    },
    EdgeCase {
        name: "ts_crlf_comment",
        file: "crlf_comment.ts",
        input: "function f() {\r
  a();\r
  // why\r
  if (x) {\r
    b();\r
  }\r
  c();\r
}\r
",
        expected: Some(
            "function f() {\r
  a();\r
\r
  // why\r
  if (x) {\r
    b();\r
  }\r
\r
  c();\r
}\r
",
        ),
    },
    EdgeCase {
        name: "ts_detached_comment",
        file: "detached_comment.ts",
        input: "function f() {
  a();
  // about a, not the if

  if (x) {
    b();
  }
}
",
        expected: None,
    },
    EdgeCase {
        name: "ts_keyword_in_comment_and_string",
        file: "keyword_in_comment_and_string.ts",
        input: "function f() {
  a(); // if this fails, return early
  const s = `
if (x) {
  return 1;
}
`;
  g(s);
}
",
        expected: None,
    },
    EdgeCase {
        name: "ts_last_comment_in_block",
        file: "last_comment_in_block.ts",
        input: "function f() {
  if (x) {
    a();
    // trailing thought
  }
  b();
}
",
        expected: Some(
            "function f() {
  if (x) {
    a();
    // trailing thought
  }

  b();
}
",
        ),
    },
    EdgeCase {
        name: "ts_multiline_prev",
        file: "multiline_prev.ts",
        input: "function f() {
  const v = compute(
    a,
    b,
  );
  if (w) {
    return v;
  }
  return v;
}
",
        expected: Some(
            "function f() {
  const v = compute(
    a,
    b,
  );

  if (w) {
    return v;
  }

  return v;
}
",
        ),
    },
    EdgeCase {
        name: "ts_no_eol",
        file: "no_eol.ts",
        input: "function f() {
  a();
  if (x) {
    b();
  }
  c();
}",
        expected: Some(
            "function f() {
  a();

  if (x) {
    b();
  }

  c();
}",
        ),
    },
    EdgeCase {
        name: "ts_setup_comment",
        file: "setup_comment.ts",
        input: "function f() {
  const x = g();
  // check it
  if (x) {
    return 1;
  }
  return 2;
}
",
        expected: Some(
            "function f() {
  const x = g();
  // check it
  if (x) {
    return 1;
  }

  return 2;
}
",
        ),
    },
    EdgeCase {
        name: "ts_stacked_comments",
        file: "stacked_comments.ts",
        input: "function f() {
  a();
  // first
  // second
  if (x) {
    b();
  }
}
",
        expected: Some(
            "function f() {
  a();

  // first
  // second
  if (x) {
    b();
  }
}
",
        ),
    },
    EdgeCase {
        name: "ts_tabs",
        file: "tabs.ts",
        input: "function f() {
\ta();
\t// tab comment
\tif (x) {
\t\tb();
\t}
}
",
        expected: Some(
            "function f() {
\ta();

\t// tab comment
\tif (x) {
\t\tb();
\t}
}
",
        ),
    },
    EdgeCase {
        name: "ts_trailing_comment_prev",
        file: "trailing_comment_prev.ts",
        input: "function f() {
  a(); // note about a
  if (x) {
    b();
  } // end if
  c();
}
",
        expected: Some(
            "function f() {
  a(); // note about a

  if (x) {
    b();
  } // end if

  c();
}
",
        ),
    },
];
