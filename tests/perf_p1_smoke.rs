//! P1 — VM hot path smoke (GetMember IC + member access + LoadGlobal IC).

use kabootar_lib::bytecode::{
    call_ic_reset_for_tests, call_ic_stats, global_ic_reset_for_tests, global_ic_stats,
    member_ic_reset_for_tests, member_ic_stats,
};
use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;
use std::sync::Mutex;

static CALL_IC_TEST: Mutex<()> = Mutex::new(());

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
    let _g = CALL_IC_TEST.lock().expect("call ic lock");
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
    let _g = CALL_IC_TEST.lock().expect("call ic lock");
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
fn call_ic_poly_two_bytecode_fns() {
    let _g = CALL_IC_TEST.lock().expect("call ic lock");
    call_ic_reset_for_tests();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        fn a() { return 1 }
        fn b() { return 2 }
        let s = 0
        let i = 0
        while i < 32 {
            let f = a
            if i > 15 {
                f = b
            }
            s = s + f()
            i = i + 1
        }
        s == 16 + 32
        "#,
        &mut env,
    )
    .expect("poly call ic");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
    let (hits, misses) = call_ic_stats();
    assert!(
        hits > misses,
        "P12b: 2-way poly should hit after filling slots, hits={hits} misses={misses}"
    );
    assert_eq!(
        kabootar_lib::bytecode::call_ic_mega_hits(),
        0,
        "two callees must not go mega"
    );
}

#[test]
fn call_ic_mega_three_bytecode_fns() {
    let _g = CALL_IC_TEST.lock().expect("call ic lock");
    call_ic_reset_for_tests();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        fn a() { return 1 }
        fn b() { return 2 }
        fn c() { return 3 }
        let s = 0
        let i = 0
        let t = 0
        while i < 32 {
            let f = a
            if t == 1 {
                f = b
            }
            if t == 2 {
                f = c
            }
            s = s + f()
            t = t + 1
            if t == 3 {
                t = 0
            }
            i = i + 1
        }
        s == 63
        "#,
        &mut env,
    )
    .expect("mega call ic");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
    let mega = kabootar_lib::bytecode::call_ic_mega_hits();
    let (hits, misses) = call_ic_stats();
    assert!(
        mega > 0,
        "P12b: 3rd distinct callee should mega-dispatch (no call_value), mega={mega} hits={hits} misses={misses}"
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
