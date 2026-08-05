//! SC2c/f autograd conv + SC4b GPU tensors / WGSL conv path.

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
fn ag_conv2d_and_sigmoid_grads() {
    let v = eval(
        r#"
        import "science"
        import "science/autograd"
        clear()
        let x = tensor([1.0, 2.0, 3.0, 4.0])
        let w = tensor([1.0, 0.0, 0.0, 1.0])
        let b = tensor([0.0])
        let y = conv2d(x, w, b, 1, 2, 2, 1, 2, 2)
        let s = sigmoid(y)
        let loss = mse(s, [0.5])
        backward(loss)
        let gx = grad(x)
        let gw = grad(w)
        typeof(gx[0]) == "number" && typeof(gw[0]) == "number" && value(y)[0] == 5.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn gpu_tensor_helpers_and_conv_kernel() {
    let v = eval(
        r#"
        import "science"
        import "science/gpu"
        let z = zeros([2, 2])
        let o = ones([2, 2])
        let s = scale(o, 3.0)
        let a = add(z, s)
        let x = toDevice(from([1.0, 2.0, 3.0, 4.0], [1, 2, 2]))
        let w = toDevice(from([1.0, 0.0, 0.0, 1.0], [1, 1, 2, 2]))
        let b = from([0.0], [1])
        let y = conv2dKernel(x, w, b, 1, 0)
        let ks = kernels()
        z["data"][0] == 0.0 && a["data"][0] == 3.0 && y["device"] == "gpu" && typeof(y["kernel"]) == "string" && y["data"][0] == 5.0 && len(ks) >= 3
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn bootstrap_exposes_autograd() {
    let v = eval(
        r#"
        import "science"
        import "science/bootstrap"
        clear()
        let t = tensor([1.0, -1.0])
        let r = relu(t)
        value(r)[0] == 1.0 && value(r)[1] == 0.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
