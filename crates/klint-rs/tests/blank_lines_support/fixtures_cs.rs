pub const CS_BEFORE_REPORT: &str = "Foo();
if (a) {
    Bar();
    for (int i = 0; i < 3; i++) {
        Baz();
    }
    foreach (var x in xs) {
        Baz();
    }
    while (a) {
        Baz();
    }
    do {
        Baz();
    } while (a);
    switch (a) {
        case 1:
            Baz();
            return 1;
    }
    try {
        Baz();
    } catch (Exception e) {
        Baz();
    } finally {
        Baz();
    }
    return 3;
}
";

pub const CS_BEFORE_CLEAN: &str = "void F() {
    if (a) {
        B();
    } else if (c) {
        D();
    } else {
        E();
    }

    while (g) {
        H();
    }
@@
    foreach (var y in g) {
        H();
    }
    // note

    switch (g) {
        case 1:
            if (a) {
                H();
            }
            break;
    }

    B(); if (c) {
        H();
    }
    var x = F();
    if (x) {
        H();
    }

    // explain
    try {
        H();
    } catch (Exception e) {
        H();
    } finally {
        H();
    }
}
";

pub const CS_AFTER_REPORT: &str = "if (a) {
    B();
}
Foo();
foreach (var x in xs) {
    if (x) {
        C();
    }
    D();
}
for (int i = 0; i < 3; i++) {
    C();
}
while (a) {
    C();
}
switch (a) {
    case 1:
        C();
        break;
}
try {
    C();
} catch (Exception e) {
    C();
}
Foo();
";

pub const CS_AFTER_CLEAN: &str = "void F() {
    if (a) {
        B();
    } else if (c) {
        D();
    } else {
        E();
    }

    foreach (var y in c) {
        if (y) {
            H();
        }
    }
@@
    while (a) {
        H();
    }

    do {
        H();
    } while (a);
    Foo();
    if (a) B();
    Foo();
    try {
        H();
    } catch (Exception e) {
        H();
    } finally {
        H();
    }

    switch (c) {
        case 1:
            if (a) {
                H();
            }

            break;
    }

    if (a) { H(); } Foo();
    while (a) {
        H();
    }
    // only a comment follows
}
";
