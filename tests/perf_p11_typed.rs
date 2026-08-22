//! P11a — typed i64 slots vs boxed `Value` locals on a tight add-loop.

use kabootar_lib::bytecode::{typed_i64_reset_for_tests, typed_i64_stats};
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;
use std::time::Instant;

const N: i64 = 80_000;

#[test]
fn typed_i64_add_loop_correct() {
    typed_i64_reset_for_tests();
    let mut env = create_global_env();
    let src = format!(
        r#"
        fn add_loop(n) {{
            let s = 0
            let i = 0
            while i < n {{
                s = s + 1
                i = i + 1
            }}
            return s
        }}
        add_loop({N})
        "#
    );
    let v = eval_source(&src, &mut env).expect("typed add_loop");
    assert!(matches!(v, Value::Number(n) if n == N), "got {v:?}");
    let (hits, _) = typed_i64_stats();
    assert!(hits > 0, "expected typed i64 path, hits={hits}");
}

#[test]
fn typed_i64_add_loop_faster_than_boxed_module() {
    typed_i64_reset_for_tests();
    let boxed_src = format!(
        r#"
        let s = 0
        let i = 0
        while i < {N} {{
            s = s + 1
            i = i + 1
        }}
        s
        "#
    );
    let typed_src = format!(
        r#"
        fn add_loop(n) {{
            let s = 0
            let i = 0
            while i < n {{
                s = s + 1
                i = i + 1
            }}
            return s
        }}
        add_loop({N})
        "#
    );

    let mut env = create_global_env();
    let t0 = Instant::now();
    let boxed = eval_source(&boxed_src, &mut env).expect("boxed loop");
    let boxed_ns = t0.elapsed().as_nanos();

    typed_i64_reset_for_tests();
    let mut env = create_global_env();
    let t1 = Instant::now();
    let typed = eval_source(&typed_src, &mut env).expect("typed loop");
    let typed_ns = t1.elapsed().as_nanos();

    assert!(matches!(boxed, Value::Number(n) if n == N), "boxed {boxed:?}");
    assert!(matches!(typed, Value::Number(n) if n == N), "typed {typed:?}");
    let (hits, _) = typed_i64_stats();
    assert!(hits > 0, "typed path not taken");
    assert!(
        typed_ns <= boxed_ns,
        "typed i64 should beat boxed module locals: typed={typed_ns} boxed={boxed_ns}"
    );
    if !cfg!(debug_assertions) {
        assert!(
            typed_ns.saturating_mul(10) <= boxed_ns,
            "P11a release gate ≥10×: typed={typed_ns} boxed={boxed_ns}"
        );
    }
}

#[test]
fn typed_f64_add_loop_correct() {
    typed_i64_reset_for_tests();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        fn fadd(n) {
            let s = 0.5
            let i = 0
            while i < n {
                s = s + 0.5
                i = i + 1
            }
            return s
        }
        fadd(4)
        "#,
        &mut env,
    )
    .expect("typed f64");
    match v {
        Value::Float(f) => assert!((f - 2.5).abs() < 1e-9, "got {f}"),
        Value::Number(n) => panic!("P11a f64 must not truncate to i64, got {n}"),
        other => panic!("got {other:?}"),
    }
    let (hits, _) = typed_i64_stats();
    assert!(hits > 0, "expected typed f64 path, hits={hits}");
}

#[test]
fn typed_bool_not_correct() {
    typed_i64_reset_for_tests();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        fn flip(b) {
            return !b
        }
        flip(true) == false && flip(false) == true
        "#,
        &mut env,
    )
    .expect("typed bool");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
    let (hits, _) = typed_i64_stats();
    assert!(hits > 0, "expected typed bool path, hits={hits}");
}

#[test]
fn typed_struct_field_acc_correct() {
    typed_i64_reset_for_tests();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        struct Pt {
            x: number;
            fn init(a) {
                self.x = a
            }
        }
        fn bump(p, n) {
            let i = 0
            while i < n {
                p.x = p.x + 1
                i = i + 1
            }
            return p.x
        }
        let p = Pt(0)
        bump(p, 8) == 8
        "#,
        &mut env,
    )
    .expect("typed struct");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
    let (hits, _) = typed_i64_stats();
    assert!(hits > 0, "expected typed struct path, hits={hits}");
}
