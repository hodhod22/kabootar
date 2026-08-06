//! Checkpoint SC: richer einsum, batch eig, polyphase, ILU/ICC, matmul HOAD, 2D PDE.

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
fn richer_einsum_and_batch_eig() {
    let v = eval(
        r#"
        import "science"
        import "science/nd"
        import "science/linalg"
        let a = from([[1.0, 2.0], [3.0, 4.0]], [2, 2])
        let b = from([[2.0, 0.0], [0.0, 3.0]], [2, 2])
        let u = from([1.0, 2.0, 3.0], [3])
        let v = from([4.0, 5.0, 6.0], [3])
        let dot = einsum("i,i->", u, v)
        let had = einsum("ij,ij->ij", a, b)
        let fro = einsum("ij,ij->", a, a)
        let rs = einsum("ij->i", a)
        let cs = einsum("ij->j", a)
        let sm = einsum("i->", u)
        let batch = [[[2.0, 0.0], [0.0, 3.0]], [[1.0, 0.0], [0.0, 1.0]]]
        let be = batchEig(batch)
        dot == 32.0 && nd_get(had, [0, 0]) == 2.0 && fro == 30.0
            && nd_get(rs, [0]) == 3.0 && nd_get(cs, [1]) == 6.0 && sm == 6.0
            && be["n"] == 2 && len(be["values"]) == 2
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn polyphase_ilu_icc() {
    let v = eval(
        r#"
        import "science"
        import "science/signal"
        import "science/sparse"
        let x = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
        let y = polyphaseResample(x, 2, 1)
        let branches = polyphaseDecompose([1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2)
        let coo = fromCoo([0, 0, 1, 1, 2, 2], [0, 1, 0, 1, 1, 2], [4.0, 1.0, 1.0, 3.0, 1.0, 2.0], 3, 3)
        let fac = ilu0(coo)
        let spd = fromCoo([0, 0, 1, 1, 1, 2, 2], [0, 1, 0, 1, 2, 1, 2], [4.0, 1.0, 1.0, 5.0, 1.0, 1.0, 3.0], 3, 3)
        let ic = icc0(spd)
        len(y) > len(x) && len(branches) == 2 && branches[0][0] == 1.0 && branches[1][0] == 2.0
            && fac["kind"] == "ilu0" && fac["l"]["format"] == "csr"
            && ic["kind"] == "icc0" && ic["l"]["nrows"] == 3
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn matmul_create_graph_and_pde2d() {
    let v = eval(
        r#"
        import "science"
        import "science/autograd"
        import "science/domain/pde"
        clear()
        let a = tensor([1.0, 0.0, 0.0, 1.0])
        let b = tensor([2.0, 3.0])
        let y = matmul(a, b, 2, 2, 1)
        let loss = sum(y)
        backward(loss, true)
        let ga = gradTensor(a)
        let gb = grad(b)
        let u0 = [[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]]
        let u1 = heat2dStep(u0, 0.1, 1.0, 1.0, 0.1)
        let f = [[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]]
        let z = [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]]
        z = poisson2dJacobi(z, f, 1.0, 1.0, 40)
        let res = poisson2dResidual(z, f, 1.0)
        typeof(ga["id"]) == "number" && gb[0] == 1.0 && gb[1] == 1.0
            && u1[1][1] < u0[1][1] && res < 0.5
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
