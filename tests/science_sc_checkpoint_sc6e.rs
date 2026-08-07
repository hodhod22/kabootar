//! Checkpoint SC: einsum path, streaming SVD, DTCWT, sparse LU/Chol, MHA HOAD, FEM BC.

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
fn einsum_path_and_streaming_svd() {
    let v = eval(
        r#"
        import "science"
        import "science/nd"
        import "science/linalg"
        let path = einsumPath("ij,jk,kl->il", [2, 3], [3, 4], [4, 2])
        let a = [[4.0, 1.0, 0.0], [1.0, 3.0, 1.0], [0.0, 1.0, 2.0]]
        let ss = streamingSvd(a, 2, 2, 2, 7)
        path["kind"] == "einsum_path" && len(path["path"]) == 2
            && ss["mode"] == "stream" && ss["rank"] == 2 && len(ss["s"]) == 2
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn dtcwt_and_sparse_lu_chol() {
    let v = eval(
        r#"
        import "science"
        import "science/signal"
        import "science/sparse"
        let x = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
        let w = dtcwt(x, 2)
        let xr = idtcwt(w["aRe"], w["aIm"], w["detailsRe"], w["detailsIm"])
        let coo = fromCoo([0, 0, 1, 1, 2, 2], [0, 1, 0, 1, 1, 2], [4.0, 1.0, 1.0, 3.0, 1.0, 2.0], 3, 3)
        let p = rcm(coo)
        let fac = lu(coo)
        let spd = fromCoo([0, 0, 1, 1, 1, 2, 2], [0, 1, 0, 1, 2, 1, 2], [4.0, 1.0, 1.0, 5.0, 1.0, 1.0, 3.0], 3, 3)
        let ch = chol(spd)
        let e0 = xr[0] - x[0]
        if e0 < 0.0 { e0 = 0.0 - e0 }
        e0 < 0.5 && w["kind"] == "dtcwt" && len(p) == 3
            && fac["kind"] == "lu" && ch["kind"] == "chol" && len(ch["p"]) == 3
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn mha_hoad_and_fem_bc() {
    let v = eval(
        r#"
        import "science"
        import "science/autograd"
        import "science/domain/pde"
        clear()
        let q = tensor([1.0, 0.0, 0.0, 1.0])
        let k = tensor([1.0, 0.0, 0.0, 1.0])
        let vv = tensor([1.0, 0.0, 0.0, 1.0])
        let y = mhaHoAd(q, k, vv, 2, 2, 1)
        backward(sum(y), true)
        let gq = gradTensor(q)
        backward(sum(gq), true)
        let g2 = grad(q)
        let nodes = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]]
        let tris = [[0, 1, 2], [1, 3, 2]]
        let edges = [[0, 1], [1, 3]]
        let km = femAssemblePoisson2dTri(nodes, tris)
        let f = femAssembleLoad2dTri(nodes, tris, 0.0)
        f = femApplyNeumann2d(nodes, edges, f, 1.0)
        let robin = femApplyRobin2d(nodes, edges, km, f, 1.0, 0.0)
        typeof(gq["id"]) == "number" && len(g2) == 4
            && f[0] > 0.0 && robin["k"][0][0] > km[0][0] - 0.0001
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
