//! P4 / P5 / P7 / P8 subset smokes.

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

#[test]
fn p5_sci_bulk_vector_ops() {
    test_runtime_env();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "science"
        let a = [1.0, 1.0, 1.0, 1.0]
        let b = [2.0, 2.0, 2.0, 2.0]
        let i = 0
        while i < 6 {
            a = sci_vadd(a, [0.0, 0.0, 0.0, 0.0])
            i = i + 1
        }
        let c = sci_vadd(a, b)
        let d = sci_vmul(a, b)
        let s = sci_dot(a, b)
        c[0] == 3.0 && d[0] == 2.0 && s == 8.0
        "#,
        &mut env,
    )
    .expect("p5");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn p7_compile_cache_second_hit() {
    use kabootar_lib::compile::{compile_file_prefer_cached, CompilePrefer};
    use std::io::Write;

    let dir = std::env::temp_dir().join("kab_p7_cache");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("p7_smoke.kab");
    let mut f = std::fs::File::create(&path).expect("create");
    writeln!(f, "1 + 1").expect("write");
    let path_s = path.to_string_lossy().to_string();
    kabootar_lib::compile::invalidate_file_cache(&path_s);
    let (_p1, b1) =
        compile_file_prefer_cached(&path_s, CompilePrefer::Rust).expect("first compile");
    let (_p2, b2) =
        compile_file_prefer_cached(&path_s, CompilePrefer::Rust).expect("second compile");
    assert!(
        b2 == "cache" || b2 == "disk-cache" || b1 == b2,
        "expected cache hit on second compile, got first={b1} second={b2}"
    );
    assert_eq!(b2, "cache", "in-memory cache should hit on second call");
}

#[test]
fn p8_job_map_sequential() {
    test_runtime_env();
    let mut env = create_global_env();
    let v = eval_source(
        r#"
        import "science"
        fn sq(x) { return x * x }
        let out = job_map([1, 2, 3, 4], sq)
        out[0] == 1 && out[3] == 16
        "#,
        &mut env,
    )
    .expect("p8");
    assert!(matches!(v, Value::Bool(true)), "got {v:?}");
}

#[test]
fn p4_aot_lite_bytecode_present() {
    // P4 subset: fingerprint `.kbc` / in-memory program bytecode is the AOT-lite path.
    use kabootar_lib::compile::compile_source;
    let src = "fn hot(x) { return x * x + 1 }\nhot(3)";
    let prog = compile_source(src).expect("compile");
    assert!(
        prog.has_bytecode(),
        "hot fn should compile to bytecode (AOT-lite cache unit)"
    );
}
