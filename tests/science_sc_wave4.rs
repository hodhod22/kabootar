//! SC1g / SC2i / SC3g / SC4e smoke.

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
fn conv2d_maxpool_embedding_mha() {
    let v = eval(
        r#"
        import "science"
        let img = nd_from([1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], [1, 3, 3])
        let w = nd_from([1.0, 0.0, 0.0, 1.0], [1, 1, 2, 2])
        let y = ml_conv2d(img, w, [0.0], 1, 0)
        let p = ml_maxpool2d(img, 2, 1)
        let emb = ml_embedding([[0.1, 0.2], [0.3, 0.4], [0.5, 0.6]], [2, 0])
        let q = nd_from([[1.0, 0.0], [0.0, 1.0]])
        let attn = ml_mha(q, q, q, 1)
        nd_shape(y)[1] == 2 && nd_shape(p)[1] == 2 && nd_shape(emb)[0] == 2 && nd_shape(attn)[0] == 2
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn ode_rk4_and_stats_plus() {
    let v = eval(
        r#"
        import "science"
        fn f(t, y) { return [0.0 - y[0]] }
        let y1 = num_rk4(f, [1.0], 0.0, 0.1)
        let traj = num_odeint(f, [1.0], 0.0, 1.0, 20)
        let q = stat_quantile([1.0, 2.0, 3.0, 4.0], 0.5)
        let tt = stat_ttest([1.0, 2.0, 3.0], [4.0, 5.0, 6.0])
        let chi = stat_chi2([10.0, 10.0], [10.0, 10.0])
        let pdf = stat_norm_pdf(0.0, 0.0, 1.0)
        y1[0] < 1.0 && len(traj["t"]) == 21 && q == 2.5 && tt["t"] < 0.0 && chi["chi2"] == 0.0 && pdf > 0.3
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn gpu_device_train_infer_path() {
    let v = eval(
        r#"
        import "science"
        let w = gpu_tensor_from([1.0, 0.0, 0.0, 1.0], [2, 2])
        let x = gpu_tensor_from([1.0, 2.0], [2])
        let wd = gpu_to_device(w)
        let xd = gpu_to_device(x)
        let y = gpu_linear(wd, xd)
        let h = gpu_to_host(y)
        let info = gpu_tensor_info()
        y["device"] == "gpu" && h["device"] == "host" && y["data"][0] == 1.0 && typeof(info["train_infer"]) == "string"
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
