//! Checkpoint SC: deepen ndarray/linalg/numerik/signal/stats/ML/physics-chem.

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
fn ndarray_reductions_transpose_tensordot() {
    let v = eval(
        r#"
        import "science"
        import "science/nd"
        let a = from([[1.0, 2.0], [3.0, 4.0]], [2, 2])
        let t = transpose(a)
        let r = roll(from([1.0, 2.0, 3.0], [3]), 1)
        let p = pad(from([1.0, 2.0], [2]), [1, 1])
        let td = tensordot(a, from([[1.0], [1.0]], [2, 1]))
        max(a) == 4.0 && min(a) == 1.0 && argmax(a) == 3
            && nd_get(t, [0, 1]) == 3.0
            && nd_get(r, [0]) == 3.0
            && nd_shape(p)[0] == 4
            && nd_shape(td)[0] == 2
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn linalg_lu_slogdet_norm() {
    let v = eval(
        r#"
        import "science"
        import "science/linalg"
        let a = [[4.0, 3.0], [6.0, 3.0]]
        let lu = lu(a)
        let sd = slogdet(a)
        let nf = normOrd(a, "fro")
        let n1 = normOrd(a, "1")
        lu["l"][1][0] > 0.0 && sd["sign"] != 0.0 && nf > 8.0 && n1 == 10.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn special_signal_numerics() {
    let v = eval(
        r#"
        import "science"
        import "science/special"
        import "science/signal"
        let e0 = erfc(0.0)
        let j1 = besselJ1(0.0)
        let g = gradient([0.0, 1.0, 4.0], 1.0)
        let f = fftfreq(4, 1.0)
        let rs = resample([0.0, 2.0], 3)
        let h = hilbert([1.0, 0.0, -1.0, 0.0])
        let sp = spectrogram([1.0, 0.0, -1.0, 0.0, 1.0, 0.0, -1.0, 0.0], 4, 2)
        e0 > 0.99 && e0 < 1.01 && j1 == 0.0
            && g[1] == 2.0 && f[1] == 0.25
            && rs[1] == 1.0 && len(h) == 8
            && sp["kind"] == "spectrogram"
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn stats_anova_mannwhitney_ppf() {
    let v = eval(
        r#"
        import "science"
        import "science/stats"
        import "science/prob"
        let a = anovaOneWay([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]])
        let mw = mannWhitney([1.0, 2.0, 3.0], [4.0, 5.0, 6.0])
        let q = normPpf(0.5, 0.0, 1.0)
        let ep = exponPpf(0.5, 1.0)
        a["f"] > 20.0 && mw["u"] == 0.0 && q > -0.01 && q < 0.01 && ep > 0.6 && ep < 0.8
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn ml_logistic_gbdt_dropout_bn_loader() {
    let v = eval(
        r#"
        import "science"
        import "science/kab_algo"
        import "science/fit"
        let X = [[0.0, 0.0], [0.1, 0.2], [0.2, 0.1], [5.0, 5.0], [5.1, 4.9], [4.8, 5.2]]
        let y = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0]
        let m = gbdtLogFitKab(X, y, 10, 0.3)
        let p = gbdtLogPredictKab(m, X)
        let d = dropoutKab([1.0, 1.0, 1.0, 1.0], 0.0, 7, true)
        let bn = batchNormKab([1.0, 2.0, 3.0], 0.0000001)
        let dl = dataLoader(6, 2, 3)
        m["kind"] == "GbdtLog" && accuracyKab(y, p) >= 0.8
            && d[0] == 1.0 && bn["mean"] == 2.0
            && len(dl["batches"]) == 3
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn physics_chem_deepen() {
    let v = eval(
        r#"
        import "science"
        import "science/mechanics"
        import "science/domain/chem"
        let ke = kineticEnergy(2.0, 3.0, 4.0)
        let pe = potentialSpring(10.0, 1.0, 0.0, 0.0, 0.0)
        let b = { "x": 0.0, "y": 0.0, "vx": 1.0, "vy": 0.0, "m": 1.0 }
        b = verletBody(b, 0.0, 0.0, 0.1)
        b = collideWall(b, -1.0, 0.05, -1.0, 1.0, 1.0)
        let pc = percentComposition("CO")
        let bonds = countBondApprox("C=C#N")
        ke == 25.0 && pe == 5.0 && b["x"] <= 0.05
            && pc["C"] > 40.0 && pc["O"] > 40.0
            && bonds["double"] == 1 && bonds["triple"] == 1
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
