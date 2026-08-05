//! SC6 deepen — beta/Bayes, ARIMA, SHAP-lite, chem.

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
fn beta_bayes_and_arima() {
    let v = eval(
        r#"
        import "science"
        import "science/prob"
        import "science/timeseries"
        let pdf = betaPdf(0.5, 2.0, 2.0)
        let cdf = betaCdf(0.5, 2.0, 2.0)
        let post = bayesBetaUpdate(1.0, 1.0, 8.0, 2.0)
        let xs = [1.0, 2.0, 3.0, 5.0, 8.0, 12.0]
        let m = arima110Fit(xs)
        let fc = arima110Forecast(m, 2)
        let m2 = arima111Fit(xs)
        let fc2 = arima111Forecast(m2, 1)
        pdf > 1.4 && cdf > 0.45 && cdf < 0.55 && post["mean"] > 0.7 && len(fc) == 2 && fc[0] > xs[len(xs)-1] && len(fc2) == 1
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn shap_and_chem() {
    let v = eval(
        r#"
        import "science"
        import "science/explain"
        import "science/domain/chem"
        let model = { "w": [2.0, -1.0], "b": 0.5, "baseline": [0.0, 0.0] }
        let sh = shapLinear(model, [1.0, 3.0])
        fn pred(row) {
            return 2.0 * row[0] - row[1]
        }
        let sk = shapKernelLite([1.0, 3.0], [0.0, 0.0], pred)
        let atoms = atomCounts(["C", "C", "O"])
        let mw = molecularWeight(["C", "C", "O"])
        let fp = atomFingerprint(["C", "C", "O"])
        sh["phi"][0] == 2.0 && sh["pred"] == -0.5 && sk["phi"][0] == 2.0 && atoms["C"] == 2 && atoms["O"] == 1 && mw > 40.0 && fp[0] == 2.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
