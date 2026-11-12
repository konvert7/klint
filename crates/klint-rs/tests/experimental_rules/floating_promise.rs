use crate::support::*;

#[test]
fn async_function_declaration_is_provable() {
    expect_promise("async function f() {}\nf();", &[2]);
}

#[test]
fn async_arrow_const_is_provable() {
    expect_promise("const f = async () => {};\nf();", &[2]);
}

#[test]
fn async_function_expression_const_is_provable() {
    expect_promise("const f = async function () {};\nf();", &[2]);
}

#[test]
fn exported_async_function_is_provable() {
    expect_promise("export async function f() {}\nf();", &[2]);
}

#[test]
fn sync_function_is_not_provable() {
    expect_promise("function f() {}\nf();", &[]);
}

#[test]
fn imported_name_is_not_provable() {
    expect_promise("import { f } from \"x\";\nf();", &[]);
}

#[test]
fn parameter_named_like_the_async_function_shadows_it() {
    expect_promise("async function f() {}\nfunction h(f) { f(); }", &[]);
}

#[test]
fn inner_sync_function_shadows_the_async_name() {
    expect_promise(
        "async function f() {}\nfunction h() { function f() {}\n f(); }",
        &[],
    );
}

#[test]
fn assignment_to_the_async_name_silences_it() {
    expect_promise("async function f() {}\nf = g;\nf();", &[]);
}

#[test]
fn assignment_inside_the_declaring_block_silences_it() {
    expect_promise("async function f() {}\nfunction h() { f = g; }\nf();", &[]);
}

#[test]
fn unrelated_parameter_named_like_the_async_name_does_not_silence_it() {
    expect_promise(
        "async function load() {}\nfunction g(load: number) {}\nload();",
        &[3],
    );
}

#[test]
fn array_destructuring_write_silences_the_async_name() {
    expect_promise("async function f() {}\n[f] = [g];\nf();", &[]);
}

#[test]
fn object_destructuring_write_silences_the_async_name() {
    expect_promise("async function f() {}\n({ f } = o);\nf();", &[]);
}

#[test]
fn call_through_a_reference_does_not_bind_the_name() {
    expect_promise("async function f() {}\nf(x);\nf();", &[2, 3]);
}

#[test]
fn unrelated_parameter_does_not_silence_the_async_name_at_top_level() {
    expect_promise("async function f() {}\nfunction h(f) {}\nf();", &[3]);
}

#[test]
fn import_of_the_async_name_shadows_it() {
    expect_promise("async function f() {}\nimport { f } from \"m\";\nf();", &[]);
}

#[test]
fn parenthesised_assignment_target_silences_the_async_name() {
    expect_promise("async function f() {}\n(f) = g;\nf();", &[]);
}

#[test]
fn shorthand_destructuring_silences_the_async_name() {
    expect_promise("async function f() {}\nconst { f } = o;\nf();", &[]);
}

#[test]
fn type_annotation_date_does_not_bind_date() {
    expect_date(
        "const a = new Date(1);\nconst d: Date = x;\nconst r = a === new Date(9);",
        &[3],
    );
}

#[test]
fn class_named_promise_makes_new_promise_unprovable() {
    expect_promise("class Promise {}\nnew Promise((r) => r(1));", &[]);
}

#[test]
fn new_promise_statement_is_reported() {
    expect_promise("new Promise((r) => r(1));", &[1]);
}

#[test]
fn promise_all_settled_statement_is_reported() {
    expect_promise("Promise.allSettled([]);", &[1]);
}

#[test]
fn promise_then_finally_chain_is_reported() {
    expect_promise("Promise.resolve(1).then((x) => x).finally(() => {});", &[1]);
}

#[test]
fn then_with_a_rejection_handler_is_silent() {
    expect_promise("async function f() {}\nf().then((x) => x, () => {});", &[]);
}

#[test]
fn catch_in_the_chain_is_silent() {
    expect_promise(
        "async function f() {}\nf().catch(() => {}).then((x) => x);",
        &[],
    );
}

#[test]
fn void_and_await_statements_are_silent() {
    expect_promise(
        "async function f() {}\nasync function g() {\n  void f();\n  await f();\n}",
        &[],
    );
}

#[test]
fn member_call_is_silent() {
    expect_promise("obj.f();", &[]);
}

#[test]
fn imported_promise_global_is_silent() {
    expect_promise(
        "import { Promise } from \"bluebird\";\nPromise.resolve(1);",
        &[],
    );
}

#[test]
fn optional_call_is_silent() {
    expect_promise("async function f() {}\nf?.();", &[]);
}

#[test]
fn plain_console_call_is_silent() {
    expect_promise("console.log(x);", &[]);
}

#[test]
fn no_floating_promise_run_reports_each_dropped_call() {
    expect_promise("async function f() {}\nf();\nf();", &[2, 3]);
}

#[test]
fn no_floating_promise_is_an_experimental_rule_that_runs() {
    expect_promise("async function f() {}\nf();", &[2]);
}
