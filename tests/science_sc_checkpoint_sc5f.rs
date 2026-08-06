//! Checkpoint SC: multi-host TCP AllReduce, GBDT, sparse cols/slice, HOAD.

use kabootar_lib::evaluator::{create_global_env, eval_source};
use kabootar_lib::value::Value;
use std::net::TcpListener;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;
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
fn tcp_allreduce_configurable_bind_host() {
    let v = eval(
        r#"
        import "science"
        import "science/dist"
        let vectors = [[1.0, 2.0], [3.0, 4.0]]
        let out = allReduceTcp(vectors, "sum", "0.0.0.0")
        out["transport"] == "tcp"
            && out["host"] == "0.0.0.0"
            && out["nRanks"] == 2
            && out["result"][0] == 4.0
            && out["result"][1] == 6.0
            && out["port"] > 0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn tcp_allreduce_multi_host_rank_api() {
    test_runtime_env();
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind probe");
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    for rank in 0..3usize {
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || -> Result<bool, String> {
            barrier.wait();
            // Stagger clients slightly after rank0 starts listening.
            if rank > 0 {
                thread::sleep(Duration::from_millis(30 * rank as u64));
            }
            // rank0: [1,4], rank1: [3,6], rank2: [5,8] -> sum [9,18]
            let code = format!(
                r#"
                import "science"
                import "science/dist"
                let local = [{x}.0, {y}.0]
                let out = allReduceTcpRank(local, "sum", {rank}, 3, "127.0.0.1", {port})
                out["result"][0] == 9.0 && out["result"][1] == 18.0 && out["rank"] == {rank}
                "#,
                x = 1 + rank * 2,
                y = 4 + rank * 2,
                rank = rank,
                port = port
            );
            let mut env = create_global_env();
            match eval_source(&code, &mut env) {
                Ok(Value::Bool(b)) => Ok(b),
                Ok(other) => Err(format!("unexpected value: {other:?}")),
                Err(e) => Err(e),
            }
        }));
    }
    let mut oks = 0;
    for h in handles {
        let ok = h.join().expect("join").expect("rank eval");
        assert!(ok, "rank assertion failed");
        oks += 1;
    }
    assert_eq!(oks, 3);
}

#[test]
fn gbdt_regression_fit_predict() {
    let v = eval(
        r#"
        import "science"
        import "science/kab_algo"
        let X = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0], [2.0, 0.0], [2.0, 1.0]]
        let y = [0.0, 2.0, 1.0, 3.0, 4.0, 5.0]
        let m = gbdtFitKab(X, y, 12, 0.25, 1)
        let p = gbdtPredictKab(m, X)
        let err = mseKab(y, p)
        m["kind"] == "Gbdt" && m["nRounds"] == 12 && err < 0.35
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn sparse_gather_cols_compress_slice() {
    let v = eval(
        r#"
        import "science"
        import "science/sparse"
        let coo = fromCoo([0, 0, 1, 2, 2], [0, 2, 1, 0, 2], [1.0, 2.0, 3.0, 4.0, 5.0], 3, 3)
        let csr = toCsr(coo)
        let g = gatherCols(csr, [2, 0])
        let c = compressCols(csr, [1.0, 0.0, 1.0])
        let s = slice(csr, 0, 2, 1, 3)
        let yg = spmv(g, [1.0, 1.0])
        g["ncols"] == 2
            && c["ncols"] == 2
            && s["nrows"] == 2
            && s["ncols"] == 2
            && yg[0] == 3.0
            && yg[2] == 9.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn autograd_higher_order_create_graph() {
    let v = eval(
        r#"
        import "science"
        import "science/autograd"
        clear()
        let x = tensor([1.0, 2.0])
        let loss = sum(exp(x))
        backward(loss, true)
        let g = gradTensor(x)
        let h = sum(g)
        backward(h)
        let hx = grad(x)
        let d0 = hx[0] - 2.718281828
        let d1 = hx[1] - 7.389056098
        if d0 < 0.0 { d0 = 0.0 - d0 }
        if d1 < 0.0 { d1 = 0.0 - d1 }
        d0 < 0.01 && d1 < 0.05
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
