pub const SWIFT_BEFORE_REPORT: &str = "foo()
if a {
    bar()
    for x in xs {
        baz()
    }
    while a {
        baz()
    }
    repeat {
        baz()
    } while a
    switch a {
    case 1:
        baz()
        return 1
    default:
        break
    }
    do {
        baz()
    } catch {
        baz()
    }
    guard a else {
        return
    }
    return 3
}
";

pub const SWIFT_BEFORE_CLEAN: &str = "func f(a: Bool, g: [Int]) {
    if a {
        b()
    } else if c {
        d()
    } else {
        e()
    }

    while a {
        h()
    }
@@
    for y in g {
        h()
    }
    // note

    switch g {
    case 1:
        if a {
            h()
        }
        break
    default:
        break
    }

    b(); if c {
        h()
    }
    let x = f()
    if x {
        h()
    }

    // explain
    do {
        h()
    } catch {
        h()
    }

    guard a else {
        return
    }
}
";

pub const SWIFT_AFTER_REPORT: &str = "if a {
    b()
}
foo()
for x in xs {
    if x {
        c()
    }
    d()
}
while a {
    c()
}
switch a {
case 1:
    c()
default:
    break
}
do {
    c()
} catch {
    c()
}
guard a else {
    return
}
foo()
";

pub const SWIFT_AFTER_CLEAN: &str = "func f(a: Bool, c: [Int]) {
    if a {
        b()
    } else if c {
        d()
    } else {
        e()
    }

    for y in c {
        if y {
            h()
        }
    }
@@
    while a {
        h()
    }

    repeat {
        h()
    } while a
    foo()
    do {
        h()
    } catch {
        h()
    }

    guard a else {
        return
    }

    switch c {
    case 1:
        if a {
            h()
        }
    default:
        break
    }

    while a {
        h()
    }
    // only a comment follows
}
";

pub const RS_BEFORE_REPORT: &str = "fn main() {
    foo();
    if a {
        bar();
        for x in xs {
            baz();
        }
        while a {
            baz();
        }
        loop {
            baz();
            break;
        }
        match a {
            1 => {
                baz();
                return;
            }
            _ => {}
        }
        if let Some(q) = a {
            baz();
        }
        while let Some(q) = a {
            baz();
        }
        return;
    }
}
";

pub const RS_BEFORE_CLEAN: &str = "fn f(a: bool, c: bool, g: Vec<i32>) -> i32 {
    if a {
        b();
    } else if c {
        d();
    } else {
        e();
    }

    while a {
        h();
    }
@@
    for y in g {
        h();
    }
    // note

    match g {
        1 => {
            if a {
                h();
            }
            h();
        }
        _ => return 2,
    }

    b(); if c {
        h();
    }
    let x = f();
    if x {
        h();
    }

    // explain
    loop {
        break;
    }
}
";

pub const RS_AFTER_REPORT: &str = "fn main() {
    if a {
        b();
    }
    foo();
    for x in xs {
        if x {
            c();
        }
        d();
    }
    while a {
        c();
    }
    loop {
        c();
        break;
    }
    match a {
        _ => {}
    }
    foo();
}
";

pub const RS_AFTER_CLEAN: &str = "fn f(a: bool, c: Vec<i32>) -> i32 {
    if a {
        b();
    } else if c {
        d();
    } else {
        e();
    }

    for y in c {
        if y {
            h();
        }
    }
@@
    while a {
        h();
    }

    loop {
        break;
    }

    match c {
        _ => {}
    }

    while a {
        h();
    }
    // only a comment follows
}
";
