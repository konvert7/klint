use super::Case;

pub const CASES: &[Case] = &[Case {
    name: "declared, assigned, property and anonymous names",
    file: "a.ts",
    source: r#"function declared(a) {
  if (a) {
    if (a) {
      g();
    }
  }
}
const assigned = (a) => {
  if (a) {
    if (a) {
      g();
    }
  }
};
const holder = {
  prop: function (a) {
    if (a) {
      if (a) {
        g();
      }
    }
  },
};
xs.forEach((a) => {
  if (a) {
    if (a) {
      g();
    }
  }
});
"#,
    expected: &[
        ("function declared", "declared", 3),
        ("const assigned", "assigned", 3),
        ("prop: function", "prop", 3),
        ("xs.forEach", "<anonymous>", 3),
    ],
}];

pub const SAME_SOURCE_EXTENSIONS: &[&str] = &["a.js", "a.jsx", "a.tsx", "a.mts", "a.cts"];

pub const EXTENSION_SOURCE: &str =
    "function f(a) {\n  if (a) {\n    if (a) {\n      g();\n    }\n  }\n}\n";
