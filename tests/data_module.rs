//! Data module MVP — DataFrame, I/O, pivot, interactive plot.

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
fn data_frame_pivot_groupby_io_plot() {
    let dir = std::env::temp_dir();
    let pq = dir.join("kab_data_mvp.kpqt");
    let pq_path = pq.to_string_lossy().replace('\\', "/");
    let code = format!(
        r#"
        import "science"
        import "data"
        os_mkdir("/data")
        let df = from([
            ["a", "x", 1.0],
            ["a", "y", 3.0],
            ["b", "x", 2.0],
            ["b", "y", 4.0]
        ], ["k", "c", "v"])
        let g = groupby(df, "k", "v", "sum")
        let pv = pivot(df, "k", "c", "v", "sum")
        writeCsv("/data/t.csv", df)
        let df2 = readCsv("/data/t.csv")
        writeJson("/data/t.json", df)
        let df3 = readJson("/data/t.json")
        writeParquet("{pq}", df)
        let df4 = readParquet("{pq}")
        let fig = interactiveLine([1.0, 2.0, 3.0], "t", 200, 100)
        let left = from([[1, "a"], [2, "b"]], ["id", "L"])
        let right = from([[2, "x"], [3, "y"]], ["id", "R"])
        let outer = join(left, right, "id", "outer")
        let leftj = join(left, right, "id", "left")
        let dt = dtypes(df)
        let barsFig = bars([1.0, 3.0, 2.0], 10, "b")
        nrows(g) == 2 && nrows(pv) == 2 && nrows(df2) == 4 && nrows(df3) == 4 && nrows(df4) == 4 && fig["mime"] == "text/html" && nrows(outer) == 3 && nrows(leftj) == 2 && dt["v"] == "float" && barsFig["type"] == "bars"
        "#,
        pq = pq_path
    );
    let v = eval(&code);
    let _ = std::fs::remove_file(&pq);
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn data_from_rows_filter() {
    let v = eval(
        r#"
        import "science"
        import "data"
        let df = fromRows([
            { "city": "a", "n": 1.0 },
            { "city": "b", "n": 5.0 },
            { "city": "a", "n": 2.0 }
        ])
        let f = filter(df, "n", ">", 1.5)
        let rows = toRows(f)
        nrows(f) == 2 && rows[0]["n"] != null
        "#,
    );
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}
