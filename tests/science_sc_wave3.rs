//! SC1f / SC2h / SC3h smoke.

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::session::Session;
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
fn pca_kmeans_logreg() {
    let v = eval(
        r#"
        import "science"
        let X = [[1.0, 1.0], [2.0, 2.0], [3.0, 3.1], [8.0, 8.0], [9.0, 8.5], [10.0, 9.5]]
        let p = ml_pca(X, 1)
        let km = ml_kmeans(X, 2, 30, 7)
        let y = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0]
        let model = ml_logreg_fit(X, y, 0.2, 80)
        let pred = ml_logreg_predict(model, [[1.0, 1.0], [9.0, 9.0]], 0.5)
        len(p["components"]) == 1 && len(km["labels"]) == 6 && pred[0] == 0 && pred[1] == 1
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn optimize_root_and_minimize() {
    let v = eval(
        r#"
        import "science"
        fn f(x) { return x * x - 2.0 }
        let r = num_root(f, 0.0, 2.0, 0.00000001, 80)
        fn g(v) { return (v[0] - 3.0) * (v[0] - 3.0) }
        let m = num_minimize(g, [0.0], 80, 0.2)
        r > 1.4 && r < 1.5 && m["x"][0] > 2.5 && m["x"][0] < 3.5
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn rich_display_table_and_plot() {
    env_host();
    let mut s = Session::new();
    s.import_science().expect("science");
    let rows = s
        .eval_cell("[[1, 2], [3, 4]]")
        .expect("rows");
    let rich = s.rich_of(&rows);
    assert_eq!(rich["ok"], true);
    assert_eq!(rich["mime"], "text/html");
    assert!(rich["html"].as_str().unwrap().contains("<table"));

    let plot = s.eval_cell("plot_line([1.0, 2.0, 1.5], 80, 40)").expect("plot");
    let rich2 = s.rich_of(&plot);
    assert!(
        rich2["mime"] == "image/png" || rich2.get("image").is_some(),
        "got {rich2}"
    );
}
