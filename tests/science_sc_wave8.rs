//! SC1i FIR/IIR + SC4b WGSL compute path + SC5b deeper Kab-port.

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
fn fir_iir_biquad() {
    let v = eval(
        r#"
        import "science"
        import "science/signal"
        let x = [1.0, 2.0, 3.0, 4.0, 5.0]
        let y = fir(x, [0.5, 0.5])
        let ma = movingAverage(x, 2)
        let z = iir(x, [1.0], [1.0, -0.5])
        let bq = biquad(x, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
        y[1] == 1.5 && ma[1] == 1.5 && z[0] == 1.0 && bq[2] == 3.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn kab_algo_deeper_port() {
    let v = eval(
        r#"
        import "science"
        import "science/kab_algo"
        let sm = softmaxKab([1.0, 2.0, 3.0])
        let lr = linregKab([1.0, 2.0, 3.0], [2.0, 4.0, 6.0])
        let d = dotKab([1.0, 2.0], [3.0, 4.0])
        let cl = clipKab([-2.0, 0.5, 9.0], 0.0, 1.0)
        let ma = movingAvgKab([1.0, 2.0, 3.0], 2)
        sm[2] > sm[0] && lr[0] > 1.9 && lr[0] < 2.1 && d == 11.0 && cl[0] == 0.0 && cl[2] == 1.0 && ma[1] == 1.5
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn gpu_matmul_kernel_path() {
    let v = eval(
        r#"
        import "science"
        import "science/gpu"
        let a = toDevice(from([1.0, 2.0, 3.0, 4.0], [2, 2]))
        let b = toDevice(from([1.0, 0.0, 0.0, 1.0], [2, 2]))
        let y = matmulKernel(a, b)
        y["device"] == "gpu" && typeof(y["kernel"]) == "string" && y["data"][0] == 1.0 && y["data"][3] == 4.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
