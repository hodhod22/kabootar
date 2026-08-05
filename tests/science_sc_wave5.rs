//! SC2j / SC2k / SC5 smoke — fit loop, tokenizer, transformer inference.

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
fn fit_linreg_and_schedulers() {
    let v = eval(
        r#"
        import "science"
        import "science/fit"
        fn step(state, epoch) {
            let X = [[1.0], [2.0], [3.0], [4.0]]
            let Y = [3.0, 5.0, 7.0, 9.0]
            let p = state["params"]
            let i = 0
            while i < 4 {
                p = ml_linreg_step(p, X[i], Y[i], 0.05)
                i = i + 1
            }
            let loss = 0.0
            let j = 0
            while j < 4 {
                let pred = p[1] + p[0] * X[j][0]
                loss = loss + (pred - Y[j]) * (pred - Y[j])
                j = j + 1
            }
            return { "state": { "params": p }, "loss": loss / 4.0 }
        }
        let res = fit({ "params": [0.0, 0.0] }, step, { "epochs": 200, "verbose": false })
        let lr1 = lrStep(1.0, 10, 0.5, 25)
        let lr2 = lrCosine(1.0, 0.0, 5, 10)
        res["state"]["params"][0] > 1.5 && res["state"]["params"][0] < 2.5 && lr1 < 1.0 && lr2 > 0.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn train_progress_rich_display() {
    env_host();
    let mut s = Session::new();
    s.import_science().expect("science");
    let log = s
        .eval_cell("ml_train_log(3, 0.42, { \"acc\": 0.9 }, { \"verbose\": false })")
        .expect("log");
    let rich = s.rich_of(&log);
    assert_eq!(rich["mime"], "text/html");
    assert!(rich["html"].as_str().unwrap().contains("epoch"));
}

#[test]
fn tokenizer_word_and_bpe() {
    let v = eval(
        r#"
        import "science"
        import "science/tokenizer"
        let vocab = buildVocab(["hello world", "hello kabootar"], 100)
        let ids = encode(vocab, "hello world")
        let text = decode(vocab, ids)
        let bpe = bpeTrain(["low low lower"], 8, 200)
        let bids = bpeEncode(bpe, "lower")
        len(ids) == 2 && text == "hello world" && len(bids) > 0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn transformer_forward_inference() {
    let v = eval(
        r#"
        import "science"
        import "science/transformer"
        let embed = nd_from([1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0], [4, 4])
        let w = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]
        let weights = {
            "embed": embed,
            "w1": w,
            "b1": [0.0, 0.0, 0.0, 0.0],
            "w2": w,
            "b2": [0.0, 0.0, 0.0, 0.0],
            "wout": w
        }
        let out = forward(weights, [1, 2], 1)
        nd_shape(out["logits"])[0] == 2 && nd_shape(out["logits"])[1] == 4 && nd_shape(out["hidden"])[1] == 4
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
