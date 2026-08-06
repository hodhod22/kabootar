//! Checkpoint SC: complex fancy index, SC5b Kab-port, multi-rank AllReduce.

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
fn complex_gather_compress_nonzero_fancy() {
    test_runtime_env();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "science"
        import "science/nd"
        let a = astype(from([1.0, 2.0, 0.0, 4.0], [4]), "complex64")
        a = set(a, 1, [3.0, 1.0])
        a = set(a, 3, [7.0, 2.0])
        let g = gather(a, [3, 1, 0])
        let mask = from([1.0, 0.0, 0.0, 1.0], [4])
        let c = compress(a, mask)
        let nz = nonzero(a)
        let m = from([1.0, 2.0, 3.0, 4.0, 5.0, 6.0], [2, 3])
        let fx = fancyIndex(m, [[0, 1], [2, 0]])
        get(g, 0)[0] == 7.0
            && get(g, 0)[1] == 2.0
            && get(g, 1)[0] == 3.0
            && nd_size(c) == 2
            && get(c, 0)[0] == 1.0
            && get(c, 1)[0] == 7.0
            && nd_size(nz) >= 2
            && nd_get(fx, 0) == 3.0
            && nd_get(fx, 1) == 4.0
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn kab_algo_sc5b_ports() {
    test_runtime_env();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "science"
        import "science/kab_algo"
        let med = medianKab([3.0, 1.0, 2.0])
        let p50 = percentileKab([0.0, 10.0, 20.0, 30.0], 50.0)
        let oh = oneHotKab(2, 4)
        let ce = crossEntropyKab([0.0, 10.0, 0.0], 1)
        let mm = matmulKab([1.0, 2.0, 3.0, 4.0], 2, 2, [1.0, 0.0, 0.0, 1.0], 2)
        let f1 = f1Kab([1.0, 1.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0])
        let cm = confusionKab([0.0, 1.0, 1.0], [0.0, 1.0, 0.0], 2)
        med == 2.0
            && p50 == 15.0
            && oh[2] == 1.0
            && oh[0] == 0.0
            && ce < 0.1
            && mm[0] == 1.0
            && mm[3] == 4.0
            && f1 > 0.0
            && cm[0][0] == 1.0
            && cm[1][1] == 1.0
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn dist_multi_rank_allreduce() {
    test_runtime_env();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "science"
        import "science/dist"
        let ranks = makeRanks([[1.0, 4.0], [3.0, 6.0], [5.0, 8.0]])
        let out = allReduceRanks(ranks, "mean", 2)
        let sum = allReduceRanks(ranks, "sum", 2)
        out["nRanks"] == 3
            && out["result"][0] == 3.0
            && out["result"][1] == 6.0
            && len(out["broadcast"]) == 3
            && out["broadcast"][2][0] == 3.0
            && sum["result"][0] == 9.0
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
