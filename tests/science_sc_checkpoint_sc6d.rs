//! Checkpoint SC: einsum ellipsis, rSVD, multilevel DWT, sparse direct, attn HOAD, FEM2d.

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
fn einsum_ellipsis_and_randomized_svd() {
    let v = eval(
        r#"
        import "science"
        import "science/nd"
        import "science/linalg"
        let batch = from([1.0, 2.0, 3.0, 4.0, 0.0, 1.0, 1.0, 0.0], [2, 2, 2])
        let vv = from([1.0, 1.0], [2])
        let out = einsum("...ij,j->...i", batch, vv)
        let a = [[4.0, 1.0, 0.0], [1.0, 3.0, 1.0], [0.0, 1.0, 2.0]]
        let rs = randomizedSvd(a, 2, 2, 7)
        nd_get(out, [0, 0]) == 3.0 && nd_get(out, [1, 0]) == 1.0
            && rs["mode"] == "rand" && rs["rank"] == 2 && len(rs["s"]) == 2
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn multilevel_dwt_and_sparse_direct() {
    let v = eval(
        r#"
        import "science"
        import "science/signal"
        import "science/sparse"
        let x = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
        let w = dwtHaarLevels(x, 2)
        let xr = idwtHaarLevels(w["a"], w["details"])
        let pkt = wptHaar(x, 2)
        let xi = iwptHaar(pkt["packets"], 2)
        let coo = fromCoo([0, 0, 1, 1, 2, 2], [0, 1, 0, 1, 1, 2], [4.0, 1.0, 1.0, 3.0, 1.0, 2.0], 3, 3)
        let fac = ilu0(coo)
        let sol = iluSolve(fac, [1.0, 2.0, 3.0])
        let spd = fromCoo([0, 0, 1, 1, 1, 2, 2], [0, 1, 0, 1, 2, 1, 2], [4.0, 1.0, 1.0, 5.0, 1.0, 1.0, 3.0], 3, 3)
        let ic = icc0(spd)
        let xs = iccSolve(ic, [1.0, 1.0, 1.0])
        let xd = spsolve(coo, [1.0, 2.0, 3.0])
        let e0 = xr[0] - x[0]
        if e0 < 0.0 { e0 = 0.0 - e0 }
        let e1 = xi[0] - x[0]
        if e1 < 0.0 { e1 = 0.0 - e1 }
        e0 < 0.000001 && e1 < 0.000001 && w["levels"] == 2 && len(pkt["packets"]) == 4
            && len(sol) == 3 && len(xs) == 3 && len(xd) == 3
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn attention_hoad_and_fem2d() {
    let v = eval(
        r#"
        import "science"
        import "science/autograd"
        import "science/domain/pde"
        clear()
        let logits = tensor([1.0, 2.0, 3.0])
        let p = softmax(logits)
        backward(sum(p), true)
        let g = gradTensor(logits)
        backward(sum(g), true)
        let g2 = grad(logits)
        clear()
        let q = tensor([1.0, 0.0, 0.0, 1.0])
        let k = tensor([1.0, 0.0, 0.0, 1.0])
        let attn = scaledDotAttn(q, k, 2, 2)
        backward(sum(attn), true)
        let gq = gradTensor(q)
        let nodes = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]]
        let tris = [[0, 1, 2], [1, 3, 2]]
        let km = femAssemblePoisson2dTri(nodes, tris)
        let f = femAssembleLoad2dTri(nodes, tris, 1.0)
        km[0][0] = 1.0
        km[0][1] = 0.0
        km[0][2] = 0.0
        km[0][3] = 0.0
        f[0] = 0.0
        let u = femSolveJacobi(km, f, 80)
        typeof(g["id"]) == "number" && len(g2) == 3
            && typeof(gq["id"]) == "number"
            && km[1][1] > 0.0 && len(f) == 4 && len(u) == 4
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
