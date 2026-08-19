//! P11–P18 tak-gates (subset). Cranelift/full nursery remain deepen.

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::runtime::ptak::{native_add_loop, nursery_reset_for_tests};
use kabootar_lib::value::Value;

#[test]
fn p11b_homogeneous_f64_sum() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let a = array_f64([1.5, 2.5, 3.0])
        array_f64_sum(a) == 7.0
        "#,
        &mut env,
    )
    .expect("f64 sum");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn p12_hidden_class_info_shared_ic() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let o = { x: 1 }
        o.y = 2
        let info = hidden_class_info()
        info["shared_ic"] == true
        "#,
        &mut env,
    )
    .expect("shape info");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn p13a_native_add_loop() {
    assert_eq!(native_add_loop(1000), 1000);
    let mut env = create_global_env();
    let v = eval_source("native_add_loop(2000) == 2000", &mut env).expect("native");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn p13a_cranelift_jit_add_loop() {
    kabootar_lib::bytecode::jit_reset_for_tests();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        fn add_loop(n) {
            let s = 0
            let i = 0
            while i < n {
                s = s + 1
                i = i + 1
            }
            return s
        }
        add_loop(5000) == 5000
        "#,
        &mut env,
    )
    .expect("jit add_loop");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
    let (hits, compiled, fails) = kabootar_lib::bytecode::jit_stats();
    assert!(
        hits + compiled > 0 || fails == 0,
        "expected cranelift JIT path hits={hits} compiled={compiled} fails={fails}"
    );
    #[cfg(not(target_arch = "wasm32"))]
    assert!(
        compiled > 0 || hits > 0,
        "Cranelift should compile or run typed add_loop, hits={hits} compiled={compiled} fails={fails}"
    );
}

#[test]
fn p14_nursery_bump() {
    nursery_reset_for_tests();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        gc_nursery_alloc(128)
        let s = gc_nursery_stats()
        let g = gc_frame_stats()
        s["allocs"] >= 1 && g["allocs"] >= 0
        "#,
        &mut env,
    )
    .expect("nursery");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn p15_manual_checks_flag() {
    let mut env = create_global_env();
    let v = eval_source("typeof(manual_checks_enabled()) == \"boolean\"", &mut env)
        .expect("manual flag");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn p16_sci_vadd_simd_chunk() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "science"
        let a = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]
        let b = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]
        let c = sci_vadd(a, b)
        len(c) == 9 && c[0] == 2.0 && c[8] == 10.0
        "#,
        &mut env,
    )
    .expect("vadd");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn p17_same_room_sql_rows_not_string() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        sql("CREATE TABLE p17_users (id INTEGER, name TEXT)")
        sql("INSERT INTO p17_users VALUES (1, 'ada')")
        let r = same_room_sql("SELECT id, name FROM p17_users")
        typeof(r) != "string" && r != null
        "#,
        &mut env,
    )
    .expect("same room");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn p18_league_and_ceiling_docs() {
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        let b = league_add_loop(20000)
        let c = tak_ceiling()
        b["python_gate"] == true && c["never"] != null && b["native"] == 20000
        "#,
        &mut env,
    )
    .expect("league");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
