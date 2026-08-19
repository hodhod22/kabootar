//! SH14 — cold / warm / incremental compiler timings (rust + product-tree cache).

use std::time::Instant;

#[test]
fn sh14_rust_compile_cold_warm() {
    let src = "fn add(a, b) { return a + b }\nreturn add(2, 3)\n";
    let t0 = Instant::now();
    let p1 = kabootar_lib::compile::compile_source(src).expect("cold");
    let cold_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let t1 = Instant::now();
    let p2 = kabootar_lib::compile::compile_source(src).expect("warm");
    let warm_ms = t1.elapsed().as_secs_f64() * 1000.0;
    eprintln!("SH14 rust compile cold={cold_ms:.2}ms warm={warm_ms:.2}ms");
    assert!(p1.bytecode.is_some() && p2.bytecode.is_some());
    let budget = if cfg!(debug_assertions) {
        5_000.0
    } else {
        500.0
    };
    assert!(
        cold_ms < budget,
        "SH14 cold rust compile {cold_ms:.1} ms exceeds {budget}"
    );
}

#[test]
fn sh14_product_tree_incremental() {
    let entry = "self_host/sample";
    let t0 = Instant::now();
    let s1 = kabootar_lib::compile::compile_dirty_product_tree(entry).expect("cold tree");
    let cold_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let t1 = Instant::now();
    let s2 = kabootar_lib::compile::compile_dirty_product_tree(entry).expect("warm tree");
    let incr_ms = t1.elapsed().as_secs_f64() * 1000.0;
    eprintln!(
        "SH14 product-tree cold_ms={cold_ms:.1} incr_ms={incr_ms:.1} dirty1={} dirty2={}",
        s1.dirty, s2.dirty
    );
    assert_eq!(s1.failed, 0);
    assert_eq!(s2.failed, 0);
    assert_eq!(s2.dirty, 0);
    assert!(
        incr_ms <= cold_ms * 2.0 + 200.0,
        "SH14 incremental should not be much slower than cold ({incr_ms:.1} vs {cold_ms:.1})"
    );
}
