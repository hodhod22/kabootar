//! SC checkpoint: broadcast/ufunc, slice/stack, QR/SVD, Adam/metrics, canvas plot.

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
fn broadcast_ufunc_and_slice_stack() {
    let v = eval(
        r#"
        import "science"
        let a = nd_from([[1.0, 2.0], [3.0, 4.0]])
        let b = nd_from([10.0, 20.0])
        let c = nd_add(a, b)
        let e = nd_exp(nd_from([0.0]))
        let s = nd_slice(a, [[0, 1], [0, 2]])
        let st = nd_stack([nd_from([1.0, 2.0]), nd_from([3.0, 4.0])], 0)
        nd_get(c, [0, 1]) == 22.0 && nd_get(e, 0) > 0.99 && nd_get(e, 0) < 1.01 && nd_shape(s)[0] == 1 && nd_shape(st)[0] == 2
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn qr_svd_cholesky_lstsq() {
    let v = eval(
        r#"
        import "science"
        let a = [[2.0, 0.0], [0.0, 3.0]]
        let q = mat_qr(a)
        let s = mat_svd(a)
        let l = mat_cholesky([[4.0, 0.0], [0.0, 9.0]])
        let x = mat_lstsq([[1.0, 1.0], [0.0, 1.0], [1.0, 0.0]], [3.0, 2.0, 1.0])
        q["r"][0][0] > 0.0 && s["s"][0] > 2.9 && l[1][1] > 2.9 && x[0] + x[1] > 2.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn adam_metrics_batch() {
    let v = eval(
        r#"
        import "science"
        let w = [1.0, 2.0]
        let g = [0.5, -0.25]
        let st = ml_adam_update(w, g, [0.0, 0.0], [0.0, 0.0], 0, 0.1)
        let acc = ml_accuracy([0, 1, 1, 0], [0, 1, 0, 0])
        let batches = ml_batch_slices(10, 4)
        let split = ml_train_test_split([[1], [2], [3], [4]], [0, 1, 0, 1], 0.5, 7)
        st["t"] == 1 && acc == 0.75 && len(batches) == 3 && len(split["x_train"]) == 2
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn canvas_plot_line() {
    let v = eval(
        r#"
        import "science"
        let p = plot_line([1.0, 3.0, 2.0, 5.0], 160, 90)
        p["kind"] == "canvas2d" && p["width"] == 160 && typeof(p["id"]) == "number"
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
