//! SC1h–j / SC2l / SC4f / SC5 smoke.

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
fn special_spline_and_signal() {
    let parts = eval(
        r#"
        import "science"
        import "science/special"
        import "science/interpolate"
        import "science/signal"
        let g = gamma(5.0)
        let e = erf(0.0)
        let j0 = besselJ0(0.0)
        let y = spline([0.0, 1.0, 2.0], [0.0, 1.0, 4.0], 1.5)
        let w = hann(8)
        let st = stft([1.0, 0.5, 0.0, -0.5, -1.0, -0.5, 0.0, 0.5, 1.0, 0.5], 4, 2)
        let f2 = fft2d([[1.0, 0.0], [0.0, 1.0]])
        [g > 4.0, e > -0.01 && e < 0.01, j0 > 0.99, y > 1.5, len(w) == 8, st["frames"] > 1, len(f2) == 2]
        "#,
    );
    let Value::Array(items) = parts else {
        panic!("expected array, got {parts:?}");
    };
    assert_eq!(items.len(), 7, "parts len");
    for (i, item) in items.iter().enumerate() {
        assert!(matches!(item, Value::Bool(true)), "part {i} failed: {item:?}");
    }
}

#[test]
fn sparse_spmv_lstsq() {
    let v = eval(
        r#"
        import "science"
        import "science/sparse"
        let coo = fromCoo([0, 1, 2], [0, 1, 2], [2.0, 3.0, 4.0], 3, 3)
        let csr = toCsr(coo)
        let y = spmv(csr, [1.0, 1.0, 1.0])
        let x = lstsq(csr, [5.0, 7.0, 9.0], 100)
        y[0] == 2.0 && y[1] == 3.0 && x[0] > 0.5
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn sci_bench_harness() {
    let v = eval(
        r#"
        import "science"
        import "science/bench"
        fn dot() { return nd_dot([1.0, 2.0], [3.0, 4.0]) }
        let b1 = bench("dot", 50, dot)
        let rep = report([b1])
        b1["iterations"] == 50 && rep["count"] == 1 && len(rep["lines"]) == 1
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn ai_delete_gate_no_python() {
    env_host();
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/science_freedom_demo.kab"
    );
    let src = std::fs::read_to_string(path).expect("read demo");
    let mut env = create_global_env();
    let v = eval_source(&src, &mut env).expect("freedom demo");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
