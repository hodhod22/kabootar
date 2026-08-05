//! SC4a BLAS API + SC2k multi-layer TF BP + SC4c Kab-closure chunks.

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
fn blas_dgemm_and_job_chunks() {
    let v = eval(
        r#"
        import "science"
        import "science/nd"
        import "science/ml"
        fn double(x) { return x * 2 }
        let gem = blasDgemm([1.0, 2.0, 3.0, 4.0], 2, 2, [1.0, 0.0, 0.0, 1.0], 2, 1.0, 0.0, null)
        let c0 = [10.0, 20.0, 30.0, 40.0]
        let gemB = sci_blas_dgemm([1.0, 0.0, 0.0, 1.0], 2, 2, [1.0, 2.0, 3.0, 4.0], 2, 2.0, 1.0, c0)
        let ch = jobMapChunks([1.0, 2.0, 3.0, 4.0], double, 2)
        gem[0] == 1.0 && gem[3] == 4.0 && gemB[0] == 12.0 && ch["data"][3] == 8.0 && ch["workers"] >= 1
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn transformer_backprop_reduces_loss() {
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
        let s0 = backpropStep(weights, ids, targets, 0.0, 1)
        let s1 = backpropStep(s0["weights"], ids, targets, 0.15, 1)
        let s2 = backpropStep(s1["weights"], ids, targets, 0.15, 1)
        s0["loss"] > 0.0 && s2["loss"] <= s0["loss"] && len(s2["layers"]) == 3
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
