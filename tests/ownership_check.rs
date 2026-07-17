//! Våg O — compile-time ownership (affine Owned + borrow) in `@manual` modules.

use kabootar_lib::bytecode::compile_source;
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn o1_use_after_move_rejected() {
    let err = compile_source(
        r#"
@manual
let b = owned_alloc(4, "t")
let c = owned_move(b)
owned_read(b, 0, 1)
"#,
    )
    .expect_err("use after move");
    assert!(err.contains("use after move"), "{err}");
}

#[test]
fn o1_assign_moves_owned() {
    let err = compile_source(
        r#"
@manual
let a = owned_alloc(4, "t")
let b = a
owned_read(a, 0, 1)
"#,
    )
    .expect_err("assign moves");
    assert!(err.contains("use after move"), "{err}");
}

#[test]
fn o1_peek_apis_do_not_move() {
    compile_source(
        r#"
@manual
let b = owned_alloc(8, "t")
owned_write(b, 0, [1, 2])
let got = owned_read(b, 0, 2)
drop(b)
return got[0] + got[1]
"#,
    )
    .expect("peek should not move");
}

#[test]
fn o2_call_arg_moves_owned() {
    let err = compile_source(
        r#"
@manual
fn take(x) {
    drop(x)
}
let b = owned_alloc(4, "t")
take(b)
owned_read(b, 0, 1)
"#,
    )
    .expect_err("call moves Owned");
    assert!(err.contains("use after move"), "{err}");
}

#[test]
fn o3_shared_borrow_keeps_owned() {
    let program = compile_source(
        r#"
@manual
fn peek(b: &Owned) {
    return owned_read(b, 0, 1)
}
let b = owned_alloc(4, "t")
owned_write(b, 0, [7])
let got = peek(&b)
drop(b)
return got[0]
"#,
    )
    .expect("shared borrow");
    let mut env = create_global_env();
    let v = eval_source(
        r#"
@manual
fn peek(b: &Owned) {
    return owned_read(b, 0, 1)
}
let b = owned_alloc(4, "t")
owned_write(b, 0, [7])
let got = peek(&b)
drop(b)
return got[0]
"#,
        &mut env,
    )
    .expect("run");
    assert!(matches!(v, Value::Number(7)), "got {v:?}");
    let _ = program;
}

#[test]
fn o3_cannot_move_while_borrowed_in_same_call() {
    // `&b` then also passing `b` in a way that moves — move after borrow in one stmt
    // is modeled as: take(b) while we also have borrow in another arg is rare;
    // moving after creating &b in sequence:
    let err = compile_source(
        r#"
@manual
fn both(r: &Owned, x: Owned) {
    drop(x)
}
let b = owned_alloc(4, "t")
both(&b, b)
"#,
    );
    // Second arg moves b while first borrowed — should error.
    let err = err.expect_err("move while borrowed");
    assert!(
        err.contains("borrow") || err.contains("use after move") || err.contains("move"),
        "{err}"
    );
}

#[test]
fn o3_double_mut_borrow_rejected() {
    let err = compile_source(
        r#"
@manual
fn poke(a: &mut Owned, b: &mut Owned) {
    owned_write(a, 0, [1])
}
let x = owned_alloc(4, "t")
poke(&mut x, &mut x)
"#,
    )
    .expect_err("double &mut");
    assert!(err.contains("&mut") || err.contains("borrow"), "{err}");
}

#[test]
fn gc_module_skips_ownership_check() {
    // Without @manual, Owned tracking does not apply (owned_alloc fails at runtime).
    compile_source(
        r#"
let a = 1
let b = a
return a + b
"#,
    )
    .expect("GC module ok");
}

#[test]
fn o4_leak_without_drop_fails() {
    let err = compile_source(
        r#"
@manual
let b = owned_alloc(4, "t")
owned_write(b, 0, [1])
"#,
    )
    .expect_err("leak without drop");
    assert!(
        err.contains("leak-lint") || err.contains("dropped out of scope"),
        "{err}"
    );
}

#[test]
fn o4_drop_clears_leak() {
    compile_source(
        r#"
@manual
let b = owned_alloc(4, "t")
drop(b)
"#,
    )
    .expect("drop clears Owned");
}

#[test]
fn r2_struct_use_after_move() {
    let err = compile_source(
        r#"
@manual
struct Point {
    x: number;
    fn init(n) { self.x = n }
}
let a = Point(1)
let b = a
let c = a
drop(b)
"#,
    )
    .expect_err("use after move");
    assert!(err.contains("use after move"), "{err}");
}

#[test]
fn r2_struct_drop_after_move_ok() {
    compile_source(
        r#"
@manual
struct Point {
    x: number;
    fn init(n) { self.x = n }
}
let a = Point(1)
let b = a
drop(b)
"#,
    )
    .expect("drop after move clears Owned");
}

#[test]
fn r2_struct_leak_without_drop() {
    let err = compile_source(
        r#"
@manual
struct Point {
    x: number;
    fn init(n) { self.x = n }
}
let a = Point(1)
"#,
    )
    .expect_err("leak without drop");
    assert!(
        err.contains("leak-lint") || err.contains("dropped out of scope"),
        "{err}"
    );
}

