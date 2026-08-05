//! SC0c / SC1c-d / SC2c / SC3 / SC4b / DX5 smoke.

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
fn nd_float64_zero_copy_roundtrip() {
    let v = eval(
        r#"
        import "science"
        let buf = array_buffer_new(32)
        let view = float64_array_new(buf, 0, 4)
        float64_array_set(view, 0, 1.5)
        float64_array_set(view, 1, 2.5)
        float64_array_set(view, 2, 3.5)
        float64_array_set(view, 3, 4.5)
        let a = nd_from_f64(view, [2, 2])
        a["zero_copy"] == true && nd_get(a, [1, 0]) == 3.5
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn fft_ifft_roundtrip_and_svd2() {
    let v = eval(
        r#"
        import "science"
        let x = [1.0, 0.0, 0.0, 0.0]
        let y = num_fft(x)
        let z = num_ifft(y)
        let s = mat_svd2([[3.0, 0.0], [0.0, 2.0]])
        z[0] > 0.99 && z[0] < 1.01 && s["s"][0] > 2.9
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn autograd_dense_mse_backward() {
    let v = eval(
        r#"
        import "science"
        ag_clear()
        let w = ag_tensor([0.5, -0.25])
        let x = ag_tensor([1.0])
        let b = ag_tensor([0.0, 0.0])
        let y = ag_dense(w, x, b)
        let loss = ag_mse(y, [1.0, 0.0])
        ag_backward(loss)
        let gw = ag_grad(w)
        gw[0] != 0.0 || gw[1] != 0.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn csv_plot_pretty_gpu_tensor() {
    let v = eval(
        r#"
        import "science"
        let rows = csv_parse("1,2\n3,4")
        let t = format_table(rows)
        let p = ascii_plot([1.0, 2.0, 1.5], 20, 5)
        let g = gpu_tensor_from([1.0, 2.0, 3.0, 4.0], [2, 2])
        let h = gpu_matmul(g, g)
        let info = gpu_tensor_info()
        len(t) > 0 && len(p) > 0 && h["shape"][0] == 2 && typeof(info["available"]) == "boolean"
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
