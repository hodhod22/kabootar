//! P1 — VM hot path smoke (GetMember IC + member access).

use kabootar_lib::bytecode::{member_ic_reset_for_tests, member_ic_stats};
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
