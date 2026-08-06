//! Checkpoint SC: einsum, batch linalg, fftn/firwin, CSC, dense HOAD, PDE.

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
fn einsum_and_batch_linalg() {
    let v = eval(
        r#"
        import "science"
        import "science/nd"
        import "science/linalg"
        let a = from([[1.0, 2.0], [3.0, 4.0]], [2, 2])
        let i2 = from([[1.0, 0.0], [0.0, 1.0]], [2, 2])
        let m = einsum("ij,jk->ik", i2, a)
        let tr = einsum("ii->", a)
        let tp = einsum("ij->ji", a)
        let batch = [[[4.0, 3.0], [6.0, 3.0]], [[2.0, 0.0], [0.0, 2.0]]]
        let bq = batchQr(batch)
        let bs = batchSvd(batch)
        let bx = batchSolve(batch, [[1.0, 0.0], [2.0, 4.0]])
        nd_get(m, [0, 1]) == 2.0 && tr == 5.0 && nd_get(tp, [0, 1]) == 3.0
            && bq["n"] == 2 && len(bs["s"]) == 2 && bx[1][0] == 1.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn fftn_firwin_butter_csc() {
    let v = eval(
        r#"
        import "science"
        import "science/signal"
        import "science/sparse"
        let fftOut = fftN([[1.0, 0.0], [0.0, 1.0]])
        let h = firwin(5, 0.2)
        let bb = butterBiquad("low", 0.1)
        let y = biquad([1.0, 1.0, 1.0, 1.0], bb["b0"], bb["b1"], bb["b2"], bb["a0"], bb["a1"], bb["a2"])
        let coo = fromCoo([0, 0, 1, 2, 2], [0, 2, 1, 0, 2], [1.0, 2.0, 3.0, 4.0, 5.0], 3, 3)
        let csc = toCsc(coo)
        let ycsr = spmv(toCsr(coo), [1.0, 1.0, 1.0])
        let ycsc = spmv(csc, [1.0, 1.0, 1.0])
        let g = gatherCols(csc, [2, 0])
        fftOut["kind"] == "fftn" && fftOut["rows"] == 2
            && h[0] + h[1] + h[2] + h[3] + h[4] > 0.99
            && len(y) == 4 && csc["format"] == "csc"
            && ycsr[0] == ycsc[0] && ycsr[2] == ycsc[2]
            && g["ncols"] == 2
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn dense_create_graph_and_pde() {
    let v = eval(
        r#"
        import "science"
        import "science/autograd"
        import "science/domain/pde"
        clear()
        let w = tensor([1.0, 2.0])
        let x = tensor([3.0, 4.0])
        let b = tensor([0.0])
        let y = dense(w, x, b)
        let loss = sum(y)
        backward(loss, true)
        let gw = gradTensor(w)
        let gwn = grad(w)
        let u0 = [0.0, 1.0, 0.5, 0.0]
        let u1 = heat1dStep(u0, 0.1, 1.0, 0.1)
        let wave = wave1dStep(u0, u0, 1.0, 1.0, 0.1)
        let f = [0.0, 2.0, 2.0, 0.0]
        let u = [0.0, 0.0, 0.0, 0.0]
        u = poisson1dJacobi(u, f, 1.0, 40)
        let res = poisson1dResidual(u, f, 1.0)
        typeof(gw["id"]) == "number" && gwn[0] == 3.0 && gwn[1] == 4.0
            && u1[1] < u0[1] && len(wave) == 4 && res < 0.5
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
