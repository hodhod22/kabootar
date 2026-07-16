//! Ownership v1 — opt-in `@manual` + owned MemBox.

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
fn manual_use_after_move_errors() {
    let code = r#"
@manual
let b1 = owned_alloc(4, "m")
let b2 = owned_move(b1)
owned_read(b1, 0, 1)
"#;
    let mut env = create_global_env();
    let err = eval_source(code, &mut env).expect_err("expected use after move");
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
