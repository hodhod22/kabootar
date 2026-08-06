//! Checkpoint SC: fancy indexing, complex64 ndarray, threaded AllReduce.

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
fn fancy_indexing_gather_and_compress() {
    test_runtime_env();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "science"
        import "science/nd"
        let a = from([10.0, 20.0, 30.0, 40.0], [4])
        let g = gather(a, [3, 1, 0])
        let mask = astype(from([1.0, 0.0, 1.0, 0.0], [4]), "bool")
        let c = compress(a, mask)
        let m = from([1.0, 2.0, 3.0, 4.0, 5.0, 6.0], [2, 3])
        let g2 = gather(m, [1, 0], 0)
        nd_get(g, 0) == 40.0
            && nd_get(g, 1) == 20.0
            && nd_get(g, 2) == 10.0
            && nd_size(c) == 2
            && nd_get(c, 0) == 10.0
            && nd_get(c, 1) == 30.0
            && nd_get(g2, [0, 0]) == 4.0
            && nd_get(g2, [1, 2]) == 3.0
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn complex64_astype_ops_and_knd_roundtrip() {
    test_runtime_env();
    let dir = std::env::temp_dir().join("kabootar_c64_sc");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("c64.knd");
    let path_s = path.to_string_lossy().replace('\\', "/");
    let mut env = create_global_env();
    let src = format!(
        r#"
        import "science"
        import "science/nd"
        let a = astype(from([1.0, 2.0], [2]), "complex64")
        let b = astype(from([3.0, 4.0], [2]), "complex64")
        b = set(b, 0, [3.0, 1.0])
        let s = add(a, b)
        let p = mul(a, b)
        let mag = abs(a)
        let cj = conj(b)
        let n = save(a, "{path}")
        let back = load("{path}")
        dtype(a) == "complex64"
            && nd_size(a) == 2
            && get(a, 0)[0] == 1.0
            && get(a, 0)[1] == 0.0
            && get(s, 0)[0] == 4.0
            && get(s, 0)[1] == 1.0
            && get(p, 0)[0] == 3.0
            && get(p, 0)[1] == 1.0
            && get(mag, 0) == 1.0
            && get(cj, 0)[1] == -1.0
            && dtype(back) == "complex64"
            && get(back, 1)[0] == 2.0
            && n == 4
        "#,
        path = path_s
    );
    let v = eval_source(&src, &mut env).expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn dist_threaded_allreduce_ops() {
    test_runtime_env();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "science"
        import "science/dist"
        let vectors = [[1.0, 10.0], [3.0, 20.0], [5.0, 30.0]]
        let mean = allReduceMean(vectors, 2)
        let sum = allReduceSum(vectors, 2)
        let mx = allReduceMax(vectors, 2)
        let via = allReduce(vectors, "mean", 2)
        mean[0] == 3.0
            && mean[1] == 20.0
            && sum[0] == 9.0
            && sum[1] == 60.0
            && mx[0] == 5.0
            && mx[1] == 30.0
            && via[0] == 3.0
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
