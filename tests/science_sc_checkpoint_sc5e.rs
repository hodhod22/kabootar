//! Checkpoint SC: TCP AllReduce, tree ensembles, sparse fancy views.

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;
use std::sync::Once;

fn test_runtime_env() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        std::env::set_var("KABOOTAR_COMPILE", "rust");
        std::env::set_var("KABOOTAR_VM", "host");
    });
}

#[test]
fn tcp_allreduce_socket_transport() {
    test_runtime_env();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "science"
        import "science/dist"
        let vectors = [[1.0, 4.0], [3.0, 6.0], [5.0, 8.0]]
        let out = allReduceTcp(vectors, "sum")
        let mean = allReduceTcp(vectors, "mean")
        out["transport"] == "tcp"
            && out["nRanks"] == 3
            && out["result"][0] == 9.0
            && out["result"][1] == 18.0
            && mean["result"][0] == 3.0
            && mean["result"][1] == 6.0
            && out["port"] > 0
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn kab_tree_ensembles() {
    test_runtime_env();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "science"
        import "science/kab_algo"
        let X = [[0.0, 0.0], [0.1, 0.2], [0.2, 0.1], [5.0, 5.0], [5.1, 4.9], [4.8, 5.2]]
        let y = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0]
        let bag = baggingStumpsKab(X, y, 7, 7)
        let fp = forestPredictKab(bag, X)
        let tree = treeKab(X, y, 3)
        let tp = treePredictKab(tree, X)
        let boost = boostStumpsKab(X, y, 6)
        let bp = boostPredictKab(boost, X)
        bag["nTrees"] == 7
            && len(fp) == 6
            && accuracyKab(y, fp) >= 0.8
            && len(tp) == 6
            && accuracyKab(y, tp) >= 0.8
            && boost["nRounds"] == 6
            && accuracyKab(y, bp) >= 0.8
            && tree != null
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn sparse_fancy_gather_compress_mask() {
    test_runtime_env();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "science"
        import "science/sparse"
        let coo = fromCoo([0, 0, 1, 2, 2], [0, 2, 1, 0, 2], [1.0, 2.0, 3.0, 4.0, 5.0], 3, 3)
        let csr = toCsr(coo)
        let g = gatherRows(csr, [2, 0])
        let c = compressRows(csr, [1.0, 0.0, 1.0])
        let dense = [[1.0, 0.0, 2.0], [0.0, 0.0, 0.0], [3.0, 4.0, 0.0]]
        let view = fromDenseMask(dense, [1.0, 0.0, 1.0])
        let y = spmv(g, [1.0, 1.0, 1.0])
        g["nrows"] == 2
            && c["nrows"] == 2
            && view["format"] == "coo"
            && view["nrows"] == 2
            && len(view["data"]) == 4
            && y[0] == 9.0
            && y[1] == 3.0
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
