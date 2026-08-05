//! SC7 deepen + SC checkpoint + DX7 + GP7 prefab smokes.

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
fn sc7_deepen_jsonl_imshow_mapnd_gpu() {
    let v = eval(
        r#"
        import "science"
        import "science/nd"
        import "science/io"
        import "science/visualize"
        import "science/parallel"
        let rows = [{ "a": 1 }, { "a": 2 }]
        let text = toJsonl(rows)
        let back = parseJsonl(text)
        let a = nd_from([0.1, 0.9, 0.2, 0.8], [2, 2])
        let im = imshowNd(a)
        let sq = mapNdParallel(a, "square", 2)
        import "science/nd_gpu"
        let eye = nd_from([1.0, 0.0, 0.0, 1.0], [2, 2])
        let g = matmulKernel(a, eye)
        let r = relu(toDevice(nd_from([-1.0, 2.0], [2])))
        let rd = toNd(r)
        let blasOk = sci_blas_backend() == "matrixmultiply"
        if sci_blas_backend() == "openblas" { blasOk = true }
        if sci_blas_backend() == "mkl" { blasOk = true }
        if sci_blas_backend() == "system_blas" { blasOk = true }
        blasOk && len(back) == 2 && back[1]["a"] == 2 && im["type"] == "imshow" && nd_size(sq) == 4 && g["shape"][0] == 2 && nd_get(rd, 0) == 0.0 && nd_get(rd, 1) == 2.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn sc_checkpoint_reinforce_shap_attn_qkv() {
    let v = eval(
        r#"
        import "science"
        import "science/rl"
        import "science/explain"
        import "science/transformer"
        fn pred(row) { return 2.0 * row[0] + 3.0 * row[1] }
        let prefs = [0.0, 0.0, 0.0]
        prefs = reinforceUpdate(prefs, 1, 1.0, 0.5)
        let sk = shapKernel([1.0, 2.0], [0.0, 0.0], pred)
        let emb = nd_from([0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8], [4, 2])
        let w = {
            "embed": emb,
            "w1": [1.0, 0.0, 0.0, 1.0],
            "w2": [1.0, 0.0, 0.0, 1.0],
            "wout": [1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            "b1": [0.0, 0.0],
            "b2": [0.0, 0.0],
            "bout": [0.0, 0.0, 0.0, 0.0],
            "wq": [1.0, 0.0, 0.0, 1.0],
            "wk": [1.0, 0.0, 0.0, 1.0],
            "wv": [1.0, 0.0, 0.0, 1.0],
            "wo": [1.0, 0.0, 0.0, 1.0]
        }
        let step = backpropStep(w, [0, 1], [1, 0], 0.01, 1)
        prefs[1] > prefs[0] && sk["coalitions"] == 4 && step["layers"][2] == "attn_qkv" && step["weights"]["wq"] != null
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn sc7_parquet_kpqt_roundtrip() {
    let dir = std::env::temp_dir();
    let path = dir.join("kab_sc7_parquet_smoke.kpqt");
    let path_s = path.to_string_lossy().replace('\\', "/");
    let v = eval(&format!(
        r#"
        import "science"
        import "science/io"
        let rows = [{{ "x": 1.5, "n": 2, "s": "a" }}, {{ "x": 3.0, "n": 4, "s": "b" }}]
        let n = parquetSave("{path_s}", rows)
        let back = parquetLoad("{path_s}")
        n == 2 && back[0]["s"] == "a" && back[1]["x"] == 3.0 && back[1]["n"] == 4
        "#
    ));
    let _ = std::fs::remove_file(&path);
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn dx7_session_helpers() {
    let v = eval(
        r#"
        import "dx/session"
        let c = commands()
        let names = listVars(["x", "_tmp", "y"])
        let s = summarizeValue({ "shape": [2], "size": 2 })
        let p = sciencePreset()
        len(c) >= 6 && len(names) == 2 && s == "ndarray" && len(p["imports"]) >= 3
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn gp7_prefab_instantiate() {
    let v = eval(
        r#"
        import "game/scene"
        import "game/editor"
        let root = createNode("root")
        let leaf = createNode("coin")
        leaf = setLocal(leaf, 1.0, 2.0, 0.0)
        let ed = createEditor(root)
        ed = refresh(ed)
        let pref = createPrefab("Coin", leaf)
        ed = instantiatePrefab(ed, pref, 3.0, 4.0, 0.0)
        ed = syncHotReload(ed, 1)
        pref["kind"] == "prefab" && len(ed["root"]["children"]) == 1 && ed["hotReload"]["stamp"] == 1
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
