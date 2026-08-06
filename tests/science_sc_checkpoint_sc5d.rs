//! Checkpoint SC: outer fancy index, mailbox AllReduce, Kab PCA/stump/kmeans.

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
fn fancy_outer_multi_axis() {
    test_runtime_env();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "science"
        import "science/nd"
        let m = from([1.0, 2.0, 3.0, 4.0, 5.0, 6.0], [2, 3])
        let o = fancyOuter(m, [[0, 1], [0, 2]])
        // shape [2,2]: (0,0)=1, (0,2)=3, (1,0)=4, (1,2)=6
        nd_shape(o)[0] == 2
            && nd_shape(o)[1] == 2
            && nd_get(o, [0, 0]) == 1.0
            && nd_get(o, [0, 1]) == 3.0
            && nd_get(o, [1, 0]) == 4.0
            && nd_get(o, [1, 1]) == 6.0
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn dist_mailbox_star_and_ring() {
    test_runtime_env();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "science"
        import "science/dist"
        let vectors = [[1.0, 10.0], [3.0, 20.0], [5.0, 30.0]]
        let c1 = createCluster(3)
        let star = allReduceStar(c1, vectors, "sum")
        let c2 = createCluster(3)
        let ring = allReduceRing(c2, vectors, "mean")
        star["ok"] == true
            && star["result"][0] == 9.0
            && star["result"][1] == 60.0
            && star["hops"] >= 2
            && ring["ok"] == true
            && ring["result"][0] == 3.0
            && ring["result"][1] == 20.0
            && ring["transport"] == "mailbox"
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn kab_pca_stump_kmeans() {
    test_runtime_env();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "science"
        import "science/kab_algo"
        let X = [[1.0, 1.0], [2.0, 2.0], [3.0, 3.0], [10.0, 0.5], [11.0, 0.0]]
        let pca = pcaKab(X, 30)
        let y = [0.0, 0.0, 0.0, 1.0, 1.0]
        let stump = stumpKab(X, y)
        let pred = stumpPredictKab(stump, X)
        let km = kmeansKab([[0.0, 0.0], [0.1, 0.1], [5.0, 5.0], [5.2, 4.8]], 2, 15)
        len(pca["component"]) == 2
            && len(pca["scores"]) == 5
            && stump["err"] <= 1.0
            && len(pred) == 5
            && km["k"] == 2
            && len(km["labels"]) == 4
            && len(km["centroids"]) == 2
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
