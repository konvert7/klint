use super::EdgeCase;

pub const CASES: [EdgeCase; 12] = [
    EdgeCase {
        name: "py_comment_before_elif",
        file: "comment_before_elif.py",
        input: "def f():
    if x:
        a()
    # otherwise
    elif y:
        b()
    try:
        c()
    # handle
    except E:
        d()
",
        expected: Some(
            "def f():
    if x:
        a()
    # otherwise
    elif y:
        b()

    try:
        c()
    # handle
    except E:
        d()
",
        ),
    },
    EdgeCase {
        name: "py_with_block",
        file: "with_block.py",
        input: "def f():
    a()
    with open(p) as h:
        b()
    c()
",
        expected: Some(
            "def f():
    a()

    with open(p) as h:
        b()

    c()
",
        ),
    },
    EdgeCase {
        name: "py_dedented_comment",
        file: "dedented_comment.py",
        input: "def f():
    if x:
        a()
    # about b
    b()
",
        expected: Some(
            "def f():
    if x:
        a()

    # about b
    b()
",
        ),
    },
    EdgeCase {
        name: "py_docstring_then_if",
        file: "docstring_then_if.py",
        input: "def f(x):
    \"\"\"Doc.\"\"\"
    if x:
        return 1
    return 2
",
        expected: Some(
            "def f(x):
    \"\"\"Doc.\"\"\"
    if x:
        return 1

    return 2
",
        ),
    },
    EdgeCase {
        name: "py_indented_trailing_comment",
        file: "indented_trailing_comment.py",
        input: "def f():
    if x:
        a()
        # still about the if body
    b()
",
        expected: Some(
            "def f():
    if x:
        a()
        # still about the if body

    b()
",
        ),
    },
    EdgeCase {
        name: "py_setup_comment",
        file: "setup_comment.py",
        input: "def f():
    x = g()
    # check it
    if x is None:
        return 1
    return x
",
        expected: Some(
            "def f():
    x = g()
    # check it
    if x is None:
        return 1

    return x
",
        ),
    },
    EdgeCase {
        name: "py_stacked_comments",
        file: "stacked_comments.py",
        input: "def f():
    a()
    # first
    # second
    if x:
        b()
",
        expected: Some(
            "def f():
    a()

    # first
    # second
    if x:
        b()
",
        ),
    },
    EdgeCase {
        name: "py_string_with_keyword",
        file: "string_with_keyword.py",
        input: "def f():
    s = \"\"\"
if x:
    return 1
\"\"\"
    g(s)
",
        expected: None,
    },
    EdgeCase {
        name: "py_trailing_comment_prev",
        file: "trailing_comment_prev.py",
        input: "def f():
    a()  # note about a
    for y in z:
        b()
    c()
",
        expected: Some(
            "def f():
    a()  # note about a

    for y in z:
        b()

    c()
",
        ),
    },
    EdgeCase {
        name: "py_class_docstring",
        file: "class_docstring.py",
        input: "class K:
    \"\"\"Doc.\"\"\"
    x = 1

def g(x):
    \"\"\"Multi
    line.\"\"\"
    for y in x:
        pass
",
        expected: None,
    },
    EdgeCase {
        name: "py_string_after_code_is_not_a_docstring",
        file: "string_after_code.py",
        input: "def f():
    a()
    \"\"\"Not a docstring.\"\"\"
    if x:
        return 1
",
        expected: Some(
            "def f():
    a()
    \"\"\"Not a docstring.\"\"\"

    if x:
        return 1
",
        ),
    },
    EdgeCase {
        name: "py_string_opening_a_loop_is_not_a_docstring",
        file: "string_in_loop.py",
        input: "def f():
    for y in z:
        \"\"\"Not a docstring.\"\"\"
        return y
",
        expected: Some(
            "def f():
    for y in z:
        \"\"\"Not a docstring.\"\"\"

        return y
",
        ),
    },
];
