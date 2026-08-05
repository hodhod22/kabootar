//! SC1g adaptive ODE/quad + SC5b kab_algo + SC4c parallel jobs + SC4a GEMM + SC2k train.

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
fn adaptive_ode_and_quad() {
    let v = eval(
        r#"
        import "science"
        import "science/optimize"
        fn f(t, y) { return [0.0 - y[0]] }
        fn g(x) { return x * x }
        let traj = odeintAdaptive(f, [1.0], 0.0, 1.0, 0.000001, 0.000001, 5000)
        let q = quad(g, 0.0, 1.0, 0.0000001, 20)
        let yend = traj["y"][len(traj["y"]) - 1][0]
        traj["n_steps"] > 1 && yend > 0.3 && yend < 0.45 && q["value"] > 0.33 && q["value"] < 0.34
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn kab_algo_deeper_and_gemm_parallel() {
    let v = eval(
        r#"
        import "science"
        import "science/kab_algo"
        import "science/ml"
        let sg = sigmoidKab([0.0])
        let acc = accuracyKab([0, 1, 1], [0, 1, 0])
        let c = corrKab([1.0, 2.0, 3.0], [2.0, 4.0, 6.0])
        let tr = trapzKab([0.0, 1.0], [0.0, 2.0])
        let nrm = normalizeL2Kab([3.0, 4.0])
        let gem = sci_gemm([1.0, 2.0, 3.0, 4.0], 2, 2, [1.0, 0.0, 0.0, 1.0], 2)
        let pm = jobMapParallel([1.0, 2.0, 3.0, 4.0], "square", 2)
        sg[0] > 0.49 && sg[0] < 0.51 && acc > 0.6 && c > 0.99 && tr == 1.0 && nrm[0] > 0.59 && nrm[0] < 0.61 && gem[0] == 1.0 && gem[3] == 4.0 && pm[2] == 9.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn transformer_train_step_reduces_loss() {
    let v = eval(
        r#"
        import "science"
        import "science/transformer"
        let embed = nd_from([1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0], [4, 4])
        let w = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]
        let weights = {
            "embed": embed,
            "w1": w,
            "b1": [0.0, 0.0, 0.0, 0.0],
            "w2": w,
            "b2": [0.0, 0.0, 0.0, 0.0],
            "wout": w,
            "bout": [0.0, 0.0, 0.0, 0.0]
        }
        let ids = [0, 1]
        let targets = [1, 2]
        let s0 = trainStep(weights, ids, targets, 0.0, 1)
        let s1 = trainStep(s0["weights"], ids, targets, 0.2, 1)
        let s2 = trainStep(s1["weights"], ids, targets, 0.2, 1)
        s0["loss"] > 0.0 && s2["loss"] <= s0["loss"]
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
