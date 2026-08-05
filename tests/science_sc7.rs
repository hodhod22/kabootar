//! SC7 surface modules — io / parallel / visualize / nd_gpu.

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
fn sc7c_io_nd_csv_json_checkpoint() {
    let path = std::env::temp_dir().join("kab_sc7_io.knd");
    let path_s = path.to_string_lossy().replace('\\', "/");
    let ck = std::env::temp_dir().join("kab_sc7_ck.json");
    let ck_s = ck.to_string_lossy().replace('\\', "/");
    let code = format!(
        r#"
        import "science"
        import "science/nd"
        import "science/io"
        let a = from([1.0, 2.0, 3.0, 4.0], [2, 2])
        let meta = writeNd(a, "{path_s}")
        let loaded = readNd("{path_s}")
        let rows = parseCsv("x,y\n1,2\n3,4")
        let js = toJson({{ "ok": true, "n": 2 }})
        let back = fromJson(js)
        saveCheckpoint("{ck_s}", [0.1, 0.2])
        let w = loadCheckpoint("{ck_s}")
        meta["size"] == 4 && loaded["size"] == 4 && len(rows) >= 2 && back["n"] == 2 && w[1] == 0.2
        "#
    );
    let v = eval(&code);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&ck);
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn sc7d_parallel_map_chunk_reduce() {
    let v = eval(
        r#"
        import "science"
        import "science/parallel"
        fn double(x) { return x * 2.0 }
        fn sumChunk(p) { return reduceSum(p) }
        fn sumList(rs) { return reduceSum(rs) }
        let xs = [1.0, 2.0, 3.0, 4.0]
        let parts = chunk(xs, 2)
        let mapped = mapItems(xs, double)
        let par = mapParallel(xs, "square", 2)
        let tot = mapReduce(xs, 2, sumChunk, sumList)
        len(parts) == 2 && len(mapped) == 4 && mapped[3] == 8.0 && len(par) == 4 && par[2] == 9.0 && tot == 10.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn sc7b_visualize_figures() {
    let v = eval(
        r#"
        import "science"
        import "science/nd"
        import "science/visualize"
        let ys = [1.0, 2.0, 3.0, 2.0, 1.0]
        let fig = lineFigure(ys, 40, 10)
        let a = from([0.1, 0.9, 0.4, 0.6], [2, 2])
        let hm = heatmapNd(a)
        let pn = plotNd(a)
        fig["kind"] == "figure" && hm["rows"] == 2 && len(hm["ascii"]) == 2 && pn["n"] == 4
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn sc7a_nd_gpu_roundtrip_matmul() {
    let v = eval(
        r#"
        import "science"
        import "science/nd"
        import "science/nd_gpu"
        let a = from([1.0, 2.0, 3.0, 4.0], [2, 2])
        let b = from([1.0, 0.0, 0.0, 1.0], [2, 2])
        let back = roundTrip(a)
        let g = matmul(a, b)
        let host = toNd(g)
        nd_get(back, 0) == 1.0 && host["size"] == 4 && nd_get(host, 0) == 1.0 && nd_get(host, 3) == 4.0
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
