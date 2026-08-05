//! SC0g–i / SC2e–f / SC3f smoke.

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
fn dtype_random_save_load() {
    let path = std::env::temp_dir().join("kab_nd_sc0i.bin");
    let path_s = path.to_string_lossy().replace('\\', "/");
    let code = format!(
        r#"
        import "science"
        nd_seed(123)
        let a = nd_rand_uniform([4], 0.0, 1.0)
        let b = nd_astype(a, "i32")
        nd_save(a, "{path_s}")
        let c = nd_load("{path_s}")
        nd_dtype(b) == "i32" && nd_size(c) == 4 && nd_get(a, 0) >= 0.0
        "#
    );
    let v = eval(&code);
    let _ = std::fs::remove_file(&path);
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn model_checkpoint_and_autograd_plus() {
    let path = std::env::temp_dir().join("kab_ml_ckpt.json");
    let path_s = path.to_string_lossy().replace('\\', "/");
    let code = format!(
        r#"
        import "science"
        ml_save_checkpoint("{path_s}", {{ "w": [1.0, 2.0], "b": [0.5] }})
        let loaded = ml_load_checkpoint("{path_s}")
        ag_clear()
        let x = ag_tensor([1.0, 2.0])
        let y = ag_softmax(x)
        let loss = ag_ce(y, [0.0, 1.0])
        ag_backward(loss)
        let gx = ag_grad(x)
        loaded["w"][0] == 1.0 && gx[0] != 0.0
        "#
    );
    let v = eval(&code);
    let _ = std::fs::remove_file(&path);
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn dataframe_select_filter_groupby() {
    let v = eval(
        r#"
        import "science"
        let df = df_from([["a", 1.0], ["b", 2.0], ["a", 3.0]], ["city", "n"])
        let f = df_filter(df, "n", ">", 1.5)
        let g = df_groupby(df, "city", "n", "sum")
        df_nrows(f) == 2 && df_nrows(g) == 2
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
