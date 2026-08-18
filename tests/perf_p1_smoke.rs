//! P1 — VM hot path smoke (GetMember IC + member access + LoadGlobal IC).

use kabootar_lib::bytecode::{
    call_ic_reset_for_tests, call_ic_stats, global_ic_reset_for_tests, global_ic_stats,
    member_ic_reset_for_tests, member_ic_stats,
};
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;

#[test]
fn member_ic_hits_on_repeated_get() {
    member_ic_reset_for_tests();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let o = { x: 1, y: 2 }
        let s = 0
        let i = 0
        while i < 32 {
            s = s + o.x
            i = i + 1
        }
        s == 32
        "#,
        &mut env,
    )
    .expect("member ic smoke");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
    let (hits, misses) = member_ic_stats();
    assert!(
        hits > 0,
        "expected GetMember IC hits after repeated access, hits={hits} misses={misses}"
    );
}

#[test]
fn global_ic_hits_on_repeated_load() {
    global_ic_reset_for_tests();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let s = 0
        let i = 0
        while i < 32 {
            s = s + len([1])
            i = i + 1
        }
        s == 32
        "#,
        &mut env,
    )
    .expect("global ic smoke");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
    let (hits, misses) = global_ic_stats();
    assert!(
        hits > 0,
        "expected LoadGlobal IC hits after repeated `len`, hits={hits} misses={misses}"
    );
}

#[test]
fn global_ic_invalidates_on_store() {
    global_ic_reset_for_tests();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let g = 1
        fn bump() {
            g = g + 1
            return g
        }
        bump() + bump() + g == 2 + 3 + 3
        "#,
        &mut env,
    )
    .expect("global ic store invalidate");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn call_ic_hits_on_repeated_native() {
    call_ic_reset_for_tests();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let s = 0
        let i = 0
        while i < 32 {
            s = s + len([1])
            i = i + 1
        }
        s == 32
        "#,
        &mut env,
    )
    .expect("call ic smoke");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
    let (hits, misses) = call_ic_stats();
    assert!(
        hits > 0,
        "expected Call IC hits on repeated native `len`, hits={hits} misses={misses}"
    );
}

#[test]
fn bytecode_call_ic_hits_on_repeated_direct() {
    call_ic_reset_for_tests();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        fn add2(a, b) {
            return a + b
        }
        let s = 0
        let i = 0
        while i < 32 {
            s = s + add2(1, 1)
            i = i + 1
        }
        s == 64
        "#,
        &mut env,
    )
    .expect("bytecode call ic");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
    let (hits, misses) = call_ic_stats();
    assert!(
        hits > 0,
        "expected Call IC hits on repeated bytecode add2, hits={hits} misses={misses}"
    );
}

#[test]
fn index_get_array_smoke() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let a = [10, 20, 30]
        a[0] + a[1] + a[2] == 60
        "#,
        &mut env,
    )
    .expect("index get");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn index_get_object_string_smoke() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let n = { "kind": "lit", "value": 3 }
        let s = 0
        let i = 0
        while i < 16 {
            s = s + n["value"]
            i = i + 1
        }
        s == 48 && n["kind"] == "lit"
        "#,
        &mut env,
    )
    .expect("object index");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn load_local_loop_smoke() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        fn sumN(n) {
            let s = 0
            let i = 0
            while i < n {
                s = s + i
                i = i + 1
            }
            return s
        }
        sumN(10) == 45
        "#,
        &mut env,
    )
    .expect("local loop");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
