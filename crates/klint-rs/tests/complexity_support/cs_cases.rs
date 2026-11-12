use super::Case;

pub const CASES: &[Case] = &[
    Case {
        name: "if else if else",
        file: "a.cs",
        source: r#"class C {
    void F(bool a, bool b) {
        if (a) {
            G();
        } else if (b) {
            G();
        } else {
            G();
        }
    }
}
"#,
        expected: &[("void F", "F", 3)],
    },
    Case {
        name: "else if inside foreach adds one with no nesting increment",
        file: "a.cs",
        source: r#"class C {
    void F(int[] xs, bool a, bool b) {
        foreach (var x in xs) {
            if (a) {
                G();
            } else if (b) {
                G();
            }
        }
    }
}
"#,
        expected: &[("void F", "F", 4)],
    },
    Case {
        name: "nested if",
        file: "a.cs",
        source: r#"class C {
    void F(bool a, bool b) {
        if (a) {
            if (b) {
                G();
            }
        }
    }
}
"#,
        expected: &[("void F", "F", 3)],
    },
    Case {
        name: "ternary",
        file: "a.cs",
        source: r#"class C {
    int F(bool a) {
        if (a) {
            G();
        }
        return a ? 1 : 2;
    }
}
"#,
        expected: &[("int F", "F", 2)],
    },
    Case {
        name: "switch statement",
        file: "a.cs",
        source: r#"class C {
    void F(int a) {
        switch (a) {
            case 1:
                G();
                break;
            default:
                H();
                break;
        }
        if (a > 0) {
            G();
        }
    }
}
"#,
        expected: &[("void F", "F", 2)],
    },
    Case {
        name: "switch expression",
        file: "a.cs",
        source: r#"class C {
    int F(int a) {
        if (a > 0) {
            G();
        }
        return a switch {
            1 => 1,
            _ => 2,
        };
    }
}
"#,
        expected: &[("int F", "F", 2)],
    },
    Case {
        name: "when clause in a switch expression arm",
        file: "a.cs",
        source: r#"class C {
    int F(int a) {
        return a switch {
            int x when x > 1 => 1,
            _ => 2,
        };
    }
}
"#,
        expected: &[("int F", "F", 2)],
    },
    Case {
        name: "pattern and and or",
        file: "a.cs",
        source: r#"class C {
    bool F(int a) {
        if (a is > 1 and < 5) {
            return true;
        }
        return false;
    }
    bool H(int a) {
        if (a > 9) {
            return false;
        }
        if (a is 1 or 2 or 3) {
            return true;
        }
        return false;
    }
    bool J(int a) {
        if (a > 9) {
            return false;
        }
        if (a is 1 or (2 or 3)) {
            return true;
        }
        return false;
    }
}
"#,
        expected: &[("bool F", "F", 2), ("bool H", "H", 2), ("bool J", "J", 2)],
    },
    Case {
        name: "goto",
        file: "a.cs",
        source: r#"class C {
    void F(bool a) {
        if (a) {
            goto done;
        }
        done:
        G();
    }
}
"#,
        expected: &[("void F", "F", 2)],
    },
    Case {
        name: "for foreach while and do",
        file: "a.cs",
        source: r#"class C {
    void F(int[] xs, bool a) {
        for (int i = 0; i < 3; i++) { G(); }
        foreach (var x in xs) { G(); }
        while (a) { G(); }
        do { G(); } while (a);
    }
}
"#,
        expected: &[("void F", "F", 4)],
    },
    Case {
        name: "catch and finally",
        file: "a.cs",
        source: r#"class C {
    void F(bool a) {
        try {
            G();
        } catch (Exception e) {
            if (a) {
                G();
            }
        } finally {
            if (a) {
                G();
            }
        }
    }
}
"#,
        expected: &[("void F", "F", 4)],
    },
    Case {
        name: "finally adds nothing and its body is not nested",
        file: "a.cs",
        source: r#"class C {
    void E1(bool a) {
        if (a) {
            G();
        }
        try {
            G();
        } finally {
        }
    }
    void E2(bool a, bool b) {
        try {
            G();
        } finally {
            if (a) {
                if (b) {
                }
            }
        }
    }
}
"#,
        expected: &[("void E2", "E2", 3)],
    },
    Case {
        name: "runs of boolean operators",
        file: "a.cs",
        source: r#"class C {
    bool R1(bool a, bool b, bool c, bool d) {
        if (a) {
            G();
        }
        return a && b && c || d;
    }
    bool R2(bool a, bool b, bool c) {
        if (a) {
            G();
        }
        return a && (b || c);
    }
    bool R3(bool a, bool b, bool c) {
        if (a) {
            G();
        }
        return a && (b || c) && a;
    }
    bool R6(bool a, bool b, bool c) {
        if (a) {
            G();
        }
        return a && (b && c);
    }
    object R4(object a, object b, object c) {
        if (a == null) {
            G();
        }
        if (b == null) {
            G();
        }
        return a ?? b ?? c;
    }
    void R5(bool a, bool b) {
        if (!a && !b) {
            G();
        }
    }
    bool R7(bool a, bool b, bool c) {
        if (a) {
            G();
        }
        return a || b || c;
    }
    bool R8(bool a, bool b, bool c) {
        if (a) {
            G();
        }
        if (b) {
            G();
        }
        return a || b || c;
    }
}
"#,
        expected: &[
            ("bool R1", "R1", 2),
            ("bool R2", "R2", 2),
            ("bool R3", "R3", 3),
            ("bool R6", "R6", 2),
            ("object R4", "R4", 2),
            ("void R5", "R5", 2),
            ("bool R8", "R8", 2),
        ],
    },
    Case {
        name: "local function inside an if starts at nesting zero",
        file: "a.cs",
        source: r#"class C {
    void F(bool a, bool b, bool c) {
        if (a) {
            void G2() {
                if (b) {
                    if (c) {
                        H();
                    }
                }
            }
            G2();
        }
        if (b) {
            H();
        }
    }
}
"#,
        expected: &[("void F", "F", 2), ("void G2", "G2", 3)],
    },
    Case {
        name: "lambda assigned to a variable",
        file: "a.cs",
        source: r#"class C {
    void F(bool a) {
        if (a) {
            G();
        }
        if (a) {
            G();
        }
        Func<bool, int> g = x => x ? 1 : 2;
    }
}
"#,
        expected: &[("void F", "F", 2)],
    },
    Case {
        name: "anonymous lambda and anonymous method",
        file: "a.cs",
        source: r#"class C {
    void F(List<bool> xs, bool a) {
        if (a) { G(); }
        if (a) { G(); }
        xs.ForEach(x => {
            if (x) {
                G();
            }
        });
        Action h = delegate () {
            if (a) {
                G();
            }
        };
    }
}
"#,
        expected: &[("void F", "F", 2)],
    },
    Case {
        name: "abstract and interface members and empty bodies are skipped",
        file: "a.cs",
        source: r#"interface I {
    void M(int a);
}
abstract class B {
    public abstract void N(int a);
    void Empty() {}
    void Real(bool a) {
        if (a) {
            G();
        }
        if (a) {
            G();
        }
    }
}
"#,
        expected: &[("void Real", "Real", 2)],
    },
    Case {
        name: "two methods on the same line are both reported in column order",
        file: "a.cs",
        source: r#"class C { void A(bool x) { if (x) { if (x) { G(); } } } void B(bool y) { if (y) { if (y) { if (y) { G(); } } } } }
"#,
        expected: &[("void A(", "A", 3), ("void A(", "B", 6)],
    },
    Case {
        name: "top-level statements are not scored",
        file: "a.cs",
        source: r#"var a = true;
if (a) {
    if (a) {
        G();
    }
}

class C {
    void Real(bool a) {
        if (a) {
            G();
        }
        if (a) {
            G();
        }
    }
}
"#,
        expected: &[("void Real", "Real", 2)],
    },
    Case {
        name: "head constructs get no nesting increment but bodies do",
        file: "a.cs",
        source: r#"class C {
    void H1(bool a, bool b, bool c) {
        if (a ? b : c) {
            G();
        }
    }
    void H2(bool a, bool b, bool c) {
        while (a ? b : c) {
            G();
        }
    }
    void H3(bool a, int[] xs, int[] ys) {
        foreach (var x in a ? xs : ys) {
            G();
        }
    }
    void H4(bool a) {
        for (int i = a ? 0 : 1; i < 3; i++) {
            G();
        }
    }
    void H5(bool a, int b, int c) {
        switch (a ? b : c) {
            case 1:
                G();
                break;
            default:
                H();
                break;
        }
    }
    int H6(bool a, bool b, bool c) {
        return (a ? b : c) ? 1 : 2;
    }
    int H7(bool a, int b, int c) {
        return (a ? b : c) switch {
            1 => 1,
            _ => 2,
        };
    }
    void H8(bool a, bool b, bool c) {
        if (a) {
            while (b ? a : c) {
                G();
            }
        }
    }
}
"#,
        expected: &[
            ("void H1", "H1", 2),
            ("void H2", "H2", 2),
            ("void H3", "H3", 2),
            ("void H4", "H4", 2),
            ("void H5", "H5", 2),
            ("int H6", "H6", 2),
            ("int H7", "H7", 2),
            ("void H8", "H8", 5),
        ],
    },
];
