//! Zero-copy nd views, slice syntax, BLAS/SIMD, stats wrappers.

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

#[test]
fn zero_copy_slice_shares_rc() {
    let v = eval(
        r#"
        import "science"
        import "science/nd"
        let a = from([1.0, 2.0, 3.0, 4.0, 5.0, 6.0], [2, 3])
        let v = slice(a, [[0, 1], [1, 3]])
        let rcA = bufRc(a)
        let rcV = bufRc(v)
        isView(v) && v["size"] == 2 && nd_get(v, 0) == 2.0 && nd_get(v, 1) == 3.0 && rcV >= 2 && rcA >= 2
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn slice_syntax_tensor_range() {
    let v = eval(
        r#"
        import "science"
        import "science/nd"
        let a = from([0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], [10])
        let v = a[1:4]
        isView(v) && v["size"] == 3 && nd_get(v, 0) == 1.0 && nd_get(v, 2) == 3.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn slice_syntax_2d_and_blas_simd() {
    let v = eval(
        r#"
        import "science"
        import "science/nd"
        let a = from([1.0, 2.0, 3.0, 4.0, 5.0, 6.0], [2, 3])
        let row = a[0:1, :]
        let gem = blasDgemm([1.0, 2.0, 3.0, 4.0], 2, 2, [1.0, 0.0, 0.0, 1.0], 2, 1.0, 0.0, null)
        let va = vadd([1.0, 2.0, 3.0, 4.0], [1.0, 1.0, 1.0, 1.0])
        isView(row) && row["size"] == 3 && gem[0] == 1.0 && gem[3] == 4.0 && va[3] == 5.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn stats_and_preprocess_metrics_nd() {
    let v = eval(
        r#"
        import "science"
        import "science/nd"
        import "science/stats"
        import "science/preprocess"
        import "science/metrics"
        let s = mean([1.0, 2.0, 3.0])
        let a = from([1.0, 2.0, 3.0, 4.0], [2, 2])
        let sc = standardScaleNd(a)
        let err = maeNd(from([1.0, 2.0], [2]), from([1.0, 3.0], [2]))
        s > 1.99 && s < 2.01 && len(sc["X"]) == 2 && err > 0.49 && err < 0.51
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
