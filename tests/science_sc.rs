//! SC0–SC2 science / AI smokes (ndarray, solve, ML).

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;
use std::sync::Once;

fn test_runtime_env() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        std::env::set_var("KABOOTAR_COMPILE", "rust");
        std::env::set_var("KABOOTAR_VM", "host");
    });
}

fn eval(code: &str) -> Value {
    test_runtime_env();
    let mut env = create_global_env();
    eval_source(code, &mut env).expect("eval")
}

#[test]
fn nd_zeros_matmul_solve() {
    let v = eval(
        r#"
        import "science"
        let a = nd_from([[2.0, 1.0], [1.0, 3.0]])
        let b = nd_from([5.0, 10.0])
        let x = nd_solve(a, b)
        let y = nd_matmul(a, nd_reshape(x, [2, 1]))
        let e0 = nd_get(y, [0, 0]) - 5.0
        let e1 = nd_get(y, [1, 0]) - 10.0
        nd_shape(a)[0] == 2 && e0 * e0 < 0.00000001 && e1 * e1 < 0.00000001
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn nd_elementwise_and_sci_vadd() {
    let v = eval(
        r#"
        import "science"
        let a = nd_ones([4])
        let b = nd_full([4], 2.0)
        let c = nd_mul(a, b)
        let d = sci_vadd([1.0, 2.0], [3.0, 4.0])
        nd_sum(c) == 8.0 && d[0] == 4.0 && sci_dot([1.0, 2.0], [3.0, 4.0]) == 11.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn ml_relu_softmax_linreg() {
    let v = eval(
        r#"
        import "science"
        let r = ml_relu([-1.0, 0.0, 2.0])
        let s = ml_softmax([1.0, 1.0])
        let params = [0.0, 0.0]
        let i = 0
        while i < 200 {
            params = ml_linreg_step(params, [1.0], 3.0, 0.2)
            i = i + 1
        }
        // pred ≈ w*1+b → 3; either weight or bias can carry the fit.
        let pred = params[0] + params[1]
        r[0] == 0.0 && r[2] == 2.0 && s[0] > 0.49 && s[0] < 0.51 && pred > 2.5 && pred < 3.5
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn ml_dense_and_job_map() {
    let v = eval(
        r#"
        import "science"
        let y = ml_dense([1.0, 2.0, 3.0, 4.0], [1.0, 1.0], [0.0, 0.0], true)
        fn double(x) { return x * 2 }
        let m = job_map([1, 2, 3], double)
        y[0] == 3.0 && y[1] == 7.0 && m[2] == 6
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn science_nd_ml_kab_wrappers() {
    let v = eval(
        r#"
        import "science"
        import "science/nd"
        import "science/ml"
        let a = from([[1.0, 0.0], [0.0, 1.0]])
        let b = from([[2.0, 3.0], [4.0, 5.0]])
        let c = matmul(a, b)
        let z = relu([-2.0, 5.0])
        nd_get(c, [1, 0]) == 4.0 && z[1] == 5.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
