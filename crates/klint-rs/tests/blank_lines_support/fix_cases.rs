pub struct FixCase {
    pub file: &'static str,
    pub before_in: &'static str,
    pub before_out: &'static str,
    pub after_in: &'static str,
    pub after_out: &'static str,
    pub gap_in: &'static str,
    pub gap_out: &'static str,
}

pub const CASES: [FixCase; 5] = [
    FixCase {
        file: "a.ts",
        before_in: "foo();
if (a) {
  b();
  // why
  for (const x of y) {
    c();
    while (d) {
      e();
    }
  }
}
",
        before_out: "foo();

if (a) {
  b();

  // why
  for (const x of y) {
    c();

    while (d) {
      e();
    }
  }
}
",
        after_in: "if (a) {
  b();
  if (c) {
    d();
    while (e) {
      f();
    }
    // note
    g();
  }
  h();
}
foo();
",
        after_out: "if (a) {
  b();
  if (c) {
    d();
    while (e) {
      f();
    }

    // note
    g();
  }

  h();
}

foo();
",
        gap_in: "if (a) {
  b();
}
while (c) {
  d();
}
",
        gap_out: "if (a) {
  b();
}

while (c) {
  d();
}
",
    },
    FixCase {
        file: "a.py",
        before_in: "foo()
if a:
    b()
    # why
    for x in y:
        c()
        while d:
            e()
",
        before_out: "foo()

if a:
    b()

    # why
    for x in y:
        c()

        while d:
            e()
",
        after_in: "if a:
    b()
    if c:
        d()
        while e:
            f()
        # note
        g()
    h()
foo()
",
        after_out: "if a:
    b()
    if c:
        d()
        while e:
            f()

        # note
        g()

    h()

foo()
",
        gap_in: "if a:
    b()
while c:
    d()
",
        gap_out: "if a:
    b()

while c:
    d()
",
    },
    FixCase {
        file: "a.swift",
        before_in: "foo()
if a {
    b()
    // why
    for x in y {
        c()
        while d {
            e()
        }
    }
}
",
        before_out: "foo()

if a {
    b()

    // why
    for x in y {
        c()

        while d {
            e()
        }
    }
}
",
        after_in: "if a {
    b()
    if c {
        d()
        while e {
            f()
        }
        // note
        g()
    }
    h()
}
foo()
",
        after_out: "if a {
    b()
    if c {
        d()
        while e {
            f()
        }

        // note
        g()
    }

    h()
}

foo()
",
        gap_in: "if a {
    b()
}
while c {
    d()
}
",
        gap_out: "if a {
    b()
}

while c {
    d()
}
",
    },
    FixCase {
        file: "a.rs",
        before_in: "fn main() {
    foo();
    if a {
        b();
        // why
        for x in y {
            c();
            while d {
                e();
            }
        }
    }
}
",
        before_out: "fn main() {
    foo();

    if a {
        b();

        // why
        for x in y {
            c();

            while d {
                e();
            }
        }
    }
}
",
        after_in: "fn main() {
    if a {
        b();
        if c {
            d();
            while e {
                f();
            }
            // note
            g();
        }
        h();
    }
    foo();
}
",
        after_out: "fn main() {
    if a {
        b();
        if c {
            d();
            while e {
                f();
            }

            // note
            g();
        }

        h();
    }

    foo();
}
",
        gap_in: "fn main() {
    if a {
        b();
    }
    while c {
        d();
    }
}
",
        gap_out: "fn main() {
    if a {
        b();
    }

    while c {
        d();
    }
}
",
    },
    FixCase {
        file: "a.cs",
        before_in: "Foo();
if (a) {
    b();
    // why
    foreach (var x in y) {
        c();
        while (d) {
            e();
        }
    }
}
",
        before_out: "Foo();

if (a) {
    b();

    // why
    foreach (var x in y) {
        c();

        while (d) {
            e();
        }
    }
}
",
        after_in: "if (a) {
    b();
    if (c) {
        d();
        while (e) {
            f();
        }
        // note
        g();
    }
    h();
}
foo();
",
        after_out: "if (a) {
    b();
    if (c) {
        d();
        while (e) {
            f();
        }

        // note
        g();
    }

    h();
}

foo();
",
        gap_in: "if (a) {
    b();
}
while (c) {
    d();
}
",
        gap_out: "if (a) {
    b();
}

while (c) {
    d();
}
",
    },
];
