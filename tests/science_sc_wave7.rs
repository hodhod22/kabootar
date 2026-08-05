//! SC2h trees/pipeline + SC2g AdamW/ROC + SC5b–e Kab-port + GPU kernel path.

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
fn stump_tree_and_pipeline() {
    let v = eval(
        r#"
        import "science"
        import "science/ml"
        import "science/pipeline"
        let X = [[0.0, 1.0], [0.2, 0.9], [0.1, 1.1], [5.0, 5.0], [5.2, 4.8], [4.9, 5.1]]
        let y = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0]
        let stump = stumpFit(X, y)
        let tree = treeFit(X, y, 2)
        let pred = treePredict(tree, [[0.0, 1.0], [5.0, 5.0]])
        fn fitTree(x, yy) { return ml_tree_fit(x, yy, 2) }
        fn predTree(model, x) { return ml_tree_predict(model, x) }
        let pipe = make([{ "name": "tree", "fit": fitTree, "predict": predTree }])
        pipe = fit(pipe, X, y)
        let p2 = predict(pipe, [[0.1, 1.0], [5.1, 5.0]])
        pred[0] == 0 && pred[1] == 1 && p2[0] == 0 && p2[1] == 1 && stump["leaf"] == false
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn adamw_roc_and_kab_algo() {
    let v = eval(
        r#"
        import "science"
        import "science/kab_algo"
        let st = ml_adamw_update([1.0, 2.0], [0.5, -0.25], [0.0, 0.0], [0.0, 0.0], 0, 0.1, 0.9, 0.999, 0.00000001, 0.01)
        let auc = ml_roc_auc([0, 0, 1, 1], [0.1, 0.4, 0.35, 0.8])
        let zs = zscore([1.0, 2.0, 3.0])
        let m = mean([2.0, 4.0])
        let rk = reluKab([-1.0, 2.0])
        st["t"] == 1 && auc > 0.5 && m == 3.0 && rk[0] == 0.0 && rk[1] == 2.0 && len(zs) == 3
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn gpu_kernel_path_and_bootstrap() {
    let v = eval(
        r#"
        import "science"
        import "science/gpu"
        import "science/bootstrap"
        let a = toDevice(from([1.0, 0.0, 0.0, 1.0], [2, 2]))
        let b = toDevice(from([1.0, 2.0, 3.0, 4.0], [2, 2]))
        let y = matmulKernel(a, b)
        let ks = kernels()
        let gi = info()
        let lr = lrStep(1.0, 10, 0.5, 20)
        y["device"] == "gpu" && typeof(y["kernel"]) == "string" && len(ks) >= 1 && len(gi["kernels"]) >= 1 && lr < 1.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
