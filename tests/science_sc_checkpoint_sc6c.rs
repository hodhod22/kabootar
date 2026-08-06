//! Checkpoint SC: einsum parser, SVD econ, wavelet banks, ILUT/IC(k), conv HOAD, 3D PDE/FEM.

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
fn einsum_parser_and_batch_svd_econ() {
    let v = eval(
        r#"
        import "science"
        import "science/nd"
        import "science/linalg"
        let a = from([[1.0, 2.0], [3.0, 4.0]], [2, 2])
        let b = from([[1.0, 0.0], [0.0, 1.0]], [2, 2])
        let c = from([[2.0, 0.0], [0.0, 3.0]], [2, 2])
        let m = einsum("ij,jk,kl->il", a, b, c)
        let bat = [[[1.0, 2.0, 3.0], [0.0, 1.0, 0.0]]]
        let bs = batchSvd(bat, "econ")
        nd_get(m, [0, 0]) == 2.0 && nd_get(m, [1, 1]) == 12.0
            && bs["mode"] == "econ" && bs["n"] == 1 && len(bs["s"][0]) == 2
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn wavelet_banks_ilut_ick() {
    let v = eval(
        r#"
        import "science"
        import "science/signal"
        import "science/sparse"
        let x = [1.0, 2.0, 3.0, 4.0]
        let w = dwtHaar(x)
        let xr = idwtHaar(w["a"], w["d"])
        let h = [0.5, 0.5, 0.5, 0.5]
        let bands = polyphaseAnalyze(x, h, 2)
        let y = polyphaseSynthesize(bands, h, 2)
        let coo = fromCoo([0, 0, 1, 1, 2, 2], [0, 1, 0, 1, 1, 2], [4.0, 1.0, 1.0, 3.0, 1.0, 2.0], 3, 3)
        let fac = ilut(coo, 0.00000001, 2.0)
        let spd = fromCoo([0, 0, 1, 1, 1, 2, 2], [0, 1, 0, 1, 2, 1, 2], [4.0, 1.0, 1.0, 5.0, 1.0, 1.0, 3.0], 3, 3)
        let ic = icK(spd, 1)
        let e0 = xr[0] - x[0]
        if e0 < 0.0 { e0 = 0.0 - e0 }
        e0 < 0.000001 && len(bands) == 2 && len(y) > 0
            && fac["kind"] == "ilut" && ic["kind"] == "ic_k" && ic["level"] == 1
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn conv_hoad_and_pde3d_fem() {
    let v = eval(
        r#"
        import "science"
        import "science/autograd"
        import "science/domain/pde"
        clear()
        let x = tensor([1.0, 2.0, 3.0, 4.0])
        let w = tensor([1.0, 0.5])
        let b = tensor([0.0, 0.0])
        let y = conv2d(x, w, b, 1, 2, 2, 2, 1, 1)
        let loss = sum(y)
        backward(loss, true)
        let gx = gradTensor(x)
        let gw = grad(w)
        clear()
        let x2 = tensor([1.0, 2.0, 3.0, 4.0])
        let w2 = tensor([1.0, 0.5])
        let b2 = tensor([0.0, 0.0])
        let y2 = conv2d(x2, w2, b2, 1, 2, 2, 2, 1, 1)
        backward(sum(y2), true)
        let gxt = gradTensor(x2)
        backward(sum(gxt), true)
        let g2w = grad(w2)
        let u0 = [[[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]], [[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]], [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]]]
        let u1 = heat3dStep(u0, 0.1, 1.0, 1.0, 1.0, 0.1)
        let f = [[[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]], [[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]], [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]]]
        let z = [[[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]], [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]], [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]]]
        z = poisson3dJacobi(z, f, 1.0, 1.0, 1.0, 40)
        let res = poisson3dResidual(z, f, 1.0)
        let km = femAssemblePoisson1d(4, 1.0)
        let uf = femSolveJacobi(km, [0.0, 1.0, 1.0, 0.0], 50)
        typeof(gx["id"]) == "number" && gw[0] == 10.0
            && typeof(gxt["id"]) == "number" && len(g2w) == 2
            && u1[1][1][1] < u0[1][1][1] && res < 0.5
            && km[0][0] == 1.0 && len(uf) == 4
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
