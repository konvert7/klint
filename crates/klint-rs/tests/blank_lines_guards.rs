mod blank_lines_support;
mod common;

use blank_lines_support::*;

fn guards_on() -> String {
    config_for(&format!("  {AFTER}:\n    guards: true\n"))
}

fn guard_lines(file: &str, source: &str) -> Vec<u64> {
    let fx = fixture("blank-guards", &guards_on(), &[(file, source)]);
    lines_of(&lint(&fx), AFTER, file)
}

#[test]
fn a_guard_followed_by_a_statement_fires_on_the_guard() {
    let source = "function f(x) {\n  if (!x) return null;\n  const y = x + 1;\n  return y;\n}\n";

    assert_eq!(guard_lines("a.ts", source), vec![2]);
}

#[test]
fn guards_are_off_by_default() {
    let source = "function f(x) {\n  if (!x) return null;\n  const y = x + 1;\n  return y;\n}\n";
    let fx = fixture("blank-guards-off", &simple(AFTER), &[("a.ts", source)]);

    assert!(lines_of(&lint(&fx), AFTER, "a.ts").is_empty());
}

#[test]
fn consecutive_guards_fire_once_after_the_last() {
    let source = "function f(x, y) {\n  if (!x) return 1;\n  if (!y) return 2;\n  use(x, y);\n}\n";

    assert_eq!(guard_lines("a.ts", source), vec![3]);
}

#[test]
fn a_guard_as_the_last_statement_does_not_fire() {
    let source = "function f(x) {\n  use(x);\n  if (!x) return;\n}\n";

    assert!(guard_lines("a.ts", source).is_empty());
}

#[test]
fn a_braceless_if_that_does_not_exit_does_not_fire() {
    let source = "function f(x) {\n  if (x) doSomething();\n  use(x);\n}\n";

    assert!(guard_lines("a.ts", source).is_empty());
}

#[test]
fn throw_continue_and_break_bodies_are_guards() {
    let source = "for (const x of xs) {\n  if (!x) continue;\n  use(x);\n  if (x > 9) break;\n  use(x);\n  if (x < 0) throw new Error(\"neg\");\n  use(x);\n}\n";

    assert_eq!(guard_lines("a.ts", source), vec![2, 4, 6]);
}

#[test]
fn an_else_if_chain_of_exits_is_one_guard() {
    let source = "function f(x) {\n  if (x === 1) return \"a\";\n  else if (x === 2) return \"b\";\n  use(x);\n}\n";

    assert_eq!(guard_lines("a.ts", source), vec![3]);
}

#[test]
fn an_else_branch_that_does_not_exit_is_not_a_guard() {
    let source = "function f(x) {\n  if (x) return 1;\n  else use(x);\n  use(x);\n}\n";

    assert!(guard_lines("a.ts", source).is_empty());
}

#[test]
fn a_csharp_guard_fires() {
    let source = "class A {\n    int F(int x) {\n        if (x < 0) throw new ArgumentException();\n        var y = x + 1;\n        return y;\n    }\n}\n";

    assert_eq!(guard_lines("A.cs", source), vec![3]);
}

#[test]
fn the_message_calls_it_a_guard() {
    let source = "function f(x) {\n  if (!x) return;\n  use(x);\n}\n";
    let fx = fixture("blank-guards-msg", &guards_on(), &[("a.ts", source)]);

    assert_eq!(
        messages(&lint(&fx), AFTER),
        vec!["Missing blank line after the `if` guard — separate it from the statement below."]
    );
}

#[test]
fn fix_keeps_setup_and_guard_together_and_separates_what_follows() {
    let config = config_for(&format!(
        "  {BEFORE}: error\n  {AFTER}:\n    guards: true\n"
    ));
    let input = "function f(id) {\n  const user = load(id);\n  if (!user) return null;\n  const name = user.name;\n  return name;\n}\n";
    let expected = "function f(id) {\n  const user = load(id);\n  if (!user) return null;\n\n  const name = user.name;\n\n  return name;\n}\n";

    fix_twice(&config, "a.ts", input, expected);
}

#[test]
fn fix_with_the_after_rule_alone_keeps_consecutive_guards_together() {
    let input =
        "function f(u) {\n  if (!u) return null;\n  if (!u.active) return null;\n  use(u);\n}\n";
    let expected =
        "function f(u) {\n  if (!u) return null;\n  if (!u.active) return null;\n\n  use(u);\n}\n";

    fix_twice(&guards_on(), "a.ts", input, expected);
}
