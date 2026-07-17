//! Ownership — `@manual` MemBox + compile-time check (Våg O).

use kabootar_lib::bytecode::compile_source;
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn manual_owned_roundtrip() {
    let code = r#"
@manual
let b = owned_alloc(8, "t")
owned_write(b, 0, [1, 2, 3])
let got = owned_read(b, 0, 3)
drop(b)
got[0] + got[1] + got[2]
"#;
    let mut env = create_global_env();
    let v = eval_source(code, &mut env).expect("manual owned roundtrip");
    assert!(matches!(v, Value::Number(6)));
}

#[test]
fn manual_use_after_move_errors_at_compile_time() {
    let code = r#"
@manual
let b1 = owned_alloc(4, "m")
let b2 = owned_move(b1)
owned_read(b1, 0, 1)
"#;
    let err = compile_source(code).expect_err("expected compile-time use after move");
    assert!(
        err.contains("use after move"),
        "unexpected error: {err}"
    );
}

#[test]
fn gc_default_rejects_owned_alloc() {
    let code = r#"
let b = owned_alloc(4, "x")
b
"#;
    let mut env = create_global_env();
    let err = eval_source(code, &mut env).expect_err("owned_alloc without @manual");
    assert!(
        err.contains("@manual") || err.contains("owned_"),
        "unexpected error: {err}"
    );
}

#[test]
fn os_display_buf_smoke() {
    let code = r#"
@manual
import "os/mem"
import "os/display_buf"
let fb = create(2, 1, "t")
fill(fb, 9, 8, 7, 6)
let px = read(fb["buf"], 0, 4)
release(fb)
px[0] + px[1] + px[2] + px[3]
"#;
    let mut env = create_global_env();
    let v = eval_source(code, &mut env).expect("display_buf smoke");
    assert!(matches!(v, Value::Number(n) if n == 30));
}

#[test]
fn manual_bytecode_owned_roundtrip() {
    use kabootar_lib::bytecode::run_module;
    let source = r#"
@manual
let b = owned_alloc(8, "t")
owned_write(b, 0, [1, 2, 3])
let got = owned_read(b, 0, 3)
drop(b)
return got[0] + got[1] + got[2]
"#;
    let program = compile_source(source).expect("compile @manual");
    let bc = program.bytecode.as_ref().expect("bytecode");
    let mut env = create_global_env();
    let v = run_module(bc, &mut env).expect("run @manual bytecode");
    assert!(matches!(v, Value::Number(6)), "got {v:?}");
}

#[test]
fn manual_overwrite_global_drops_old() {
    use kabootar_lib::bytecode::run_module;
    let source = r#"
@manual
let b = owned_alloc(4, "a")
b = owned_alloc(4, "b")
owned_write(b, 0, [9])
let got = owned_read(b, 0, 1)
drop(b)
return got[0]
"#;
    let program = compile_source(source).expect("compile overwrite");
    let bc = program.bytecode.as_ref().expect("bytecode");
    let mut env = create_global_env();
    let v = run_module(bc, &mut env).expect("run overwrite");
    assert!(matches!(v, Value::Number(9)), "got {v:?}");
}

#[test]
fn os_mem_move_wrapper() {
    let code = r#"
@manual
import "os/mem"
let b = alloc(4, "m")
let c = move(b)
owned_read(b, 0, 1)
"#;
    let err = compile_source(code).expect_err("use after move via mem.move");
    assert!(
        err.contains("use after move"),
        "unexpected error: {err}"
    );
}
