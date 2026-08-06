//! Checkpoint SC: MKL thread control, multi-layer TF stack, nested Parquet.

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
fn blas_thread_control_and_info() {
    test_runtime_env();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "science"
        import "science/nd"
        let info0 = blasInfo()
        let set = blasSetNumThreads(2)
        let n = blasNumThreads()
        let backend = blasBackend()
        info0["backend"] == backend
            && n >= 1
            && set["threads"] == 2
            && (backend == "matrixmultiply" || backend == "openblas" || backend == "mkl" || backend == "system_blas")
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn tf_multi_layer_stack_forward_and_backprop() {
    test_runtime_env();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "science"
        import "science/transformer"
        // Tiny LM: vocab=4, d=4, one head, two stacked blocks via nLayers.
        let d = 4
        let vsz = 4
        let eye = []
        let i = 0
        while i < d * d {
            eye = push(eye, 0.0)
            i = i + 1
        }
        i = 0
        while i < d {
            eye[i * d + i] = 1.0
            i = i + 1
        }
        let embedData = []
        i = 0
        while i < vsz * d {
            embedData = push(embedData, 0.01 * i)
            i = i + 1
        }
        let w1 = []
        i = 0
        while i < d * d {
            w1 = push(w1, 0.05)
            i = i + 1
        }
        let wout = []
        i = 0
        while i < vsz * d {
            wout = push(wout, 0.02)
            i = i + 1
        }
        let block = {
            "wq": eye, "wk": eye, "wv": eye, "wo": eye,
            "w1": w1, "b1": [0.0, 0.0, 0.0, 0.0],
            "w2": w1, "b2": [0.0, 0.0, 0.0, 0.0]
        }
        let weights = {
            "embed": { "__kab_nd": true, "shape": [vsz, d], "data": embedData, "dtype": "f64", "size": vsz * d },
            "wout": wout,
            "bout": [0.0, 0.0, 0.0, 0.0],
            "layers": [block, block],
            "wq": eye, "wk": eye, "wv": eye, "wo": eye,
            "w1": w1, "b1": [0.0, 0.0, 0.0, 0.0],
            "w2": w1, "b2": [0.0, 0.0, 0.0, 0.0]
        }
        let ids = [0, 1, 2]
        let fwd1 = forward(weights, ids, 1)
        let fwd2 = stackForward(weights, ids, 1, 2)
        let step = stackBackpropStep(weights, ids, [1, 2, 3], 0.01, 1, 2)
        fwd2["nLayers"] == 2
            && step["stack"] == true
            && step["nLayers"] == 2
            && step["loss"] > 0.0
            && fwd1["hidden"]["shape"][0] == 3
            && fwd2["hidden"]["shape"][0] == 3
        "#,
        &mut env,
    )
    .expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn nested_parquet_list_and_struct_roundtrip() {
    test_runtime_env();
    let dir = std::env::temp_dir().join("kabootar_nested_parquet_sc");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("nested.parquet");
    let path_s = path.to_string_lossy().replace('\\', "/");
    let mut env = create_global_env();
    let src = format!(
        r#"
        import "science"
        import "science/io"
        let rows = [
            {{ "id": 1, "vals": [1.0, 2.0, 3.0], "meta": {{ "k": "a", "n": "1" }} }},
            {{ "id": 2, "vals": [4.0, 5.0], "meta": {{ "k": "b", "n": "2" }} }}
        ]
        let n = writeParquet("{path}", rows)
        let back = readParquet("{path}")
        n == 2
            && len(back) == 2
            && len(back[0]["vals"]) == 3
            && back[0]["vals"][0] == 1.0
            && back[1]["vals"][1] == 5.0
            && back[0]["meta"]["k"] == "a"
            && back[1]["meta"]["k"] == "b"
        "#,
        path = path_s
    );
    let v = eval_source(&src, &mut env).expect("eval");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
