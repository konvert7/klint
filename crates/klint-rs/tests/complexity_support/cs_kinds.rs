use super::{ANY_NAME, Case};

pub const CASES: &[Case] = &[Case {
    name: "constructor destructor accessor and operators",
    file: "a.cs",
    source: r#"class C {
    int v;
    public C(bool a) {
        if (a) {
            if (a) {
                G();
            }
        }
    }
    ~C() {
        if (v > 0) {
            if (v > 1) {
                G();
            }
        }
    }
    public int V {
        get {
            if (v > 0) {
                if (v > 1) {
                    return 1;
                }
            }
            return 0;
        }
    }
    public static C operator +(C x, C y) {
        if (x.v > 0) {
            if (y.v > 0) {
                G();
            }
        }
        return x;
    }
    public static implicit operator int(C x) {
        if (x.v > 0) {
            if (x.v > 1) {
                G();
            }
        }
        return 0;
    }
}
"#,
    expected: &[
        ("public C(bool a)", ANY_NAME, 3),
        ("~C()", ANY_NAME, 3),
        ("        get {", ANY_NAME, 3),
        ("operator +", ANY_NAME, 3),
        ("implicit operator", ANY_NAME, 3),
    ],
}];
