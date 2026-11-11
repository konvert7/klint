pub const TS_BEFORE_REPORT: &str = "foo();
if (foo) {
  bar();
  for (const x of xs) {
    baz();
  }
  for (const k in ks) {
    baz();
  }
  while (a) {
    baz();
  }
  do {
    baz();
  } while (a);
  switch (a) {
    case 1:
      baz();
      return 1;
  }
  try {
    baz();
  } catch (e) {
    baz();
  } finally {
    baz();
  }
  return 3;
}
";

pub const TS_BEFORE_CLEAN: &str = "function f(a, c, g) {
  if (a) {
    b();
  } else if (c) {
    d();
  } else {
    e();
  }

  while (g) {
    h();
  }
@@
  for (const y of g) {
    h();
  }
  // note

  switch (g) {
    case 1:
      if (a) {
        h();
      }
      break;
  }

  b(); if (c) {
    h();
  }
  const x = f();
  if (x) {
    h();
  }

  // explain
  try {
    h();
  } catch (e) {
    h();
  } finally {
    h();
  }
}

function g(a) {
  if (a) return 1;
}
";

pub const TS_AFTER_REPORT: &str = "if (a) {
  b();
}
foo();
for (const x of xs) {
  if (x) {
    c();
  }
  d();
}
for (const k in ks) {
  c();
}
while (a) {
  c();
}
switch (a) {
  case 1:
    c();
}
try {
  c();
} catch (e) {
  c();
}
foo();
";

pub const TS_AFTER_CLEAN: &str = "function f(a, c) {
  if (a) {
    b();
  } else if (c) {
    d();
  } else {
    e();
  }

  for (const y of c) {
    if (y) {
      h();
    }
  }
@@
  while (a) {
    h();
  }

  do {
    h();
  } while (a);
  foo();
  if (a) b();
  foo();
  try {
    h();
  } catch (e) {
    h();
  } finally {
    h();
  }

  switch (c) {
    case 1:
      if (a) {
        h();
      }
  }

  if (a) { h(); } foo();
  while (a) {
    h();
  }
  // only a comment follows
}
";

pub const PY_BEFORE_REPORT: &str = "foo()
if a:
    bar()
    for x in xs:
        baz()
    while a:
        baz()
    match a:
        case 1:
            baz()
            return 1
    try:
        baz()
    except E:
        baz()
    finally:
        baz()
    with open(f) as g:
        baz()
    return 3
";

pub const PY_BEFORE_CLEAN: &str = "def f(a, c, g):
    if a:
        b()
    elif c:
        d()
    else:
        e()

    while g:
        h()
@@
    for y in g:
        h()
    # note

    match g:
        case 1:
            if a:
                h()
            pass

    b(); x = 1
    x = f()
    if x:
        h()

    # explain
    try:
        h()
    except E:
        h()
    finally:
        h()


def g(a):
    if a: return 1
    y = 1; return y
";

pub const PY_AFTER_REPORT: &str = "if a:
    b()
x = 1
for i in xs:
    if i:
        c()
    d()
while a:
    c()
match a:
    case 1:
        c()
try:
    c()
except E:
    c()
with f as g:
    c()
foo()
";

pub const PY_AFTER_CLEAN: &str = "def f(a, c):
    if a:
        b()
    elif c:
        d()
    else:
        e()

    for y in c:
        if y:
            h()
@@
    while a:
        h()

    try:
        h()
    except E:
        h()
    finally:
        h()

    match c:
        case 1:
            if a:
                h()

    foo()
    while a:
        h()
    # only a comment follows
";
