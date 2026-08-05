//! SC6 production modules — prob/preprocess/metrics + timeseries/explain/graph/rl/dist/domain.

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
fn sc6abc_prob_preprocess_metrics() {
    let v = eval(
        r#"
        import "science"
        import "science/prob"
        import "science/preprocess"
        import "science/metrics"
        let p = poissonPdf(2, 2.0)
        let e = exponCdf(1.0, 1.0)
        let ci = bootstrapCi([1.0, 2.0, 3.0, 4.0], 40, 0.2, 1.0)
        let X = [[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]]
        let sc = standardScale(X)
        let oh = oneHot([0, 1, 0], 2)
        let pr = precision([1, 0, 1, 1], [1, 0, 0, 1])
        let rc = recall([1, 0, 1, 1], [1, 0, 0, 1])
        let r = r2([1.0, 2.0, 3.0], [1.1, 1.9, 3.1])
        p > 0.2 && e > 0.6 && ci["low"] <= ci["high"] && sc["X"][0][0] < 0.0 && oh[1][1] == 1.0 && pr > 0.9 && rc > 0.6 && r > 0.9
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn sc6e_g_timeseries_explain() {
    let v = eval(
        r#"
        import "science"
        import "science/timeseries"
        import "science/explain"
        let xs = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
        let a = acf(xs, 1)
        let ar = ar1Fit(xs)
        let fc = ar1Forecast(ar, 6.0, 2)
        let heat = confusionHeat([0, 1, 1], [0, 1, 0], 2)
        let imps = corrImportance([[1.0], [2.0], [3.0]], [2.0, 4.0, 6.0])
        a >= 0.5 && ar["b"] > 0.5 && len(fc) == 2 && len(heat["ascii"]) == 2 && imps[0] > 0.9
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn sc6d_f_h_graph_rl_dist() {
    let v = eval(
        r#"
        import "science"
        import "science/graph"
        import "science/rl"
        import "science/dist"
        let g = fromEdges(3, [[0, 1], [1, 2]])
        let feat = degreeFeatures(g)
        let agg = meanAggregate(g, [[1.0], [2.0], [3.0]])
        let env = createEnv(3, 2, { "0:1": 1.0, "1:1": 1.0 }, { "0:1": 1, "1:1": 2 })
        env = reset(env)
        let st = step(env, 1)
        let q = {}
        q = qLearnUpdate(q, 0, 1, 1.0, 1, 0.5, 0.9, 2)
        let chunks = chunk([1.0, 2.0, 3.0, 4.0], 2)
        let avg = allReduceMean([[1.0, 2.0], [3.0, 4.0]])
        let pm = parallelMapF64([1.0, 2.0, 3.0], "square", 2)
        degree(g, 1) == 2 && feat[1][0] > 0.0 && agg[1][0] > 1.0 && st["state"] == 1 && q["0:1"] > 0.0 && len(chunks) == 2 && avg[0] == 2.0 && pm[2] == 9.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn sc6i_domain_modules() {
    let v = eval(
        r#"
        import "science"
        import "science/domain/finance"
        import "science/domain/bio"
        import "science/tokenizer"
        import "science/domain/nlp"
        let rets = returns([100.0, 110.0, 121.0])
        let gc = gcContent(["A", "T", "G", "C"])
        let vocab = buildVocab(["hello world"], 50)
        let bow = bagOfWords(vocab, "hello world")
        rets[0] > 0.09 && rets[0] < 0.11 && gc == 0.5 && (bow[0] + bow[1] + bow[2] + bow[3]) >= 2.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
