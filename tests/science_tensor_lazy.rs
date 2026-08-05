//! Tensor buffer ownership (`nd_take`) + GC lazy realize.

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;
use std::sync::Once;

fn env_host() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        std::env::set_var("KABOOTAR_COMPILE", "rust");
        std::env::set_var("KABOOTAR_VM", "host");
    });
}

fn eval(code: &str) -> Value {
    env_host();
    let mut env = create_global_env();
    eval_source(code, &mut env).expect("eval")
}

fn eval_err(code: &str) -> String {
    env_host();
    let mut env = create_global_env();
    eval_source(code, &mut env).expect_err("expected error")
}

#[test]
fn tensor_unique_owner_and_take_move() {
    let v = eval(
        r#"
        import "science"
        import "science/nd"
        import "science/tensor"
        let t = Tensor([1.0, 2.0, 3.0, 4.0], [2, 2])
        let own0 = isOwner(t)
        let u = take(t)
        let moved = isMoved(t)
        let own1 = isOwner(u)
        let x = nd_get(u, 0)
        own0 && moved && own1 && x == 1.0 && u["size"] == 4
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn take_rejects_shared_view() {
    let err = eval_err(
        r#"
        import "science"
        import "science/nd"
        let a = ensureOwned(from([1.0, 2.0, 3.0, 4.0], [2, 2]))
        let v = slice(a, [[0, 1], [0, 2]])
        take(a)
        "#,
    );
    assert!(
        err.contains("shared") || err.contains("view"),
        "unexpected err: {err}"
    );
}

#[test]
fn use_after_move_errors() {
    let err = eval_err(
        r#"
        import "science"
        import "science/nd"
        import "science/tensor"
        let t = Tensor([1.0, 2.0], [2])
        let u = take(t)
        nd_get(t, 0)
        "#,
    );
    assert!(err.contains("move") || err.contains("moved"), "unexpected err: {err}");
}

#[test]
fn lazy_realize_add_matmul_memo() {
    let v = eval(
        r#"
        import "science"
        import "science/nd"
        import "science/lazy"
        let a = lazy(from([1.0, 2.0, 3.0, 4.0], [2, 2]))
        let b = lazy(from([1.0, 0.0, 0.0, 1.0], [2, 2]))
        let c = lazyAdd(a, b)
        let d = lazyMatmul(c, b)
        let r1 = realize(d)
        let r2 = realize(d)
        let ok = r1["size"] == 4 && nd_get(r1, 0) == 2.0 && nd_get(r1, 3) == 5.0
        ok && d["done"] == true && r2["size"] == 4
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn tensor_meta_is_gc() {
    let v = eval(
        r#"
        import "science"
        import "science/nd"
        import "science/tensor"
        let t = Tensor([1.0, 2.0], [2])
        setMeta(t, "name", "w")
        getMeta(t, "name") == "w" && isTensor(t) && isOwner(t)
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
