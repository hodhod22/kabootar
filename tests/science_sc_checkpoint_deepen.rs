//! SC deepen: broadcastTo, autograd sub/div/sum/exp, QR/SVD modes, rfft.

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
fn broadcast_to_and_shapes() {
    let v = eval(
        r#"
        import "science"
        import "science/nd"
        let a = from([1.0, 2.0, 3.0], [1, 3])
        let b = broadcastTo(a, [2, 3])
        let sh = broadcastShapes([3, 1], [1, 4])
        nd_get(b, [1, 2]) == 3.0 && sh[0] == 3 && sh[1] == 4 && nd_shape(b)[0] == 2
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn autograd_sub_div_sum_exp() {
    let v = eval(
        r#"
        import "science"
        import "science/autograd"
        clear()
        let x = tensor([1.0, 2.0])
        let y = tensor([0.5, 1.0])
        let z = sub(mul(exp(x), y), div(x, y))
        let loss = sum(z)
        backward(loss)
        let gx = grad(x)
        typeof(gx[0]) == "number" && gx[0] != 0.0 && value(loss)[0] > 0.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn qr_svd_modes_and_pinv() {
    let v = eval(
        r#"
        import "science"
        import "science/linalg"
        let a = [[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]]
        let thin = qr(a)
        let full = qr(a, "full")
        let err = qrErr(a)
        let s = svd(a, "thin")
        let sf = svd(a, "full")
        let p = pinv(a)
        thin["mode"] == "thin" && full["mode"] == "full" && err < 0.00000001
            && len(s["s"]) == 2 && len(sf["u"]) == 3 && len(sf["u"][0]) == 3
            && len(p) == 2 && len(p[0]) == 3
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn rfft_irfft_roundtrip() {
    let v = eval(
        r#"
        import "science"
        import "science/signal"
        let x = [1.0, 0.0, -1.0, 0.0]
        let spec = rfft(x)
        let y = irfft(spec)
        let pad = fftPad([1.0, 2.0, 3.0])
        let c = fftC([1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
        let d0 = y[0] - 1.0
        let d2 = y[2] + 1.0
        if d0 < 0.0 { d0 = 0.0 - d0 }
        if d2 < 0.0 { d2 = 0.0 - d2 }
        d0 < 0.000000001 && d2 < 0.000000001
            && len(spec) == 6 && len(pad) == 8 && len(c) == 8
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
