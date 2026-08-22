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

/// SH14 — rust-host `compile_source` is already gated above. Self-host cold/warm
/// (session reuse after SH13) lives in `sh_wave::sh8_tiny_self_host_compile` so CI
/// pays one toolchain import, then a second `compile()` on the same thread.

#[test]
fn sh14_rust_compile_medium_loc() {
    let n = if cfg!(debug_assertions) { 800 } else { 10_000 };
    let mut src = String::with_capacity(n * 12 + 24);
    src.push_str("let s = 0\n");
    for _ in 0..n {
        src.push_str("s = s + 1\n");
    }
    src.push_str("s\n");
    let loc = n + 2;
    let t0 = Instant::now();
    let p = kabootar_lib::compile::compile_source(&src).expect("medium loc");
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    eprintln!("SH14 rust compile loc={loc} ms={ms:.1}");
    assert!(p.bytecode.is_some());
    let budget = if cfg!(debug_assertions) {
        45_000.0
    } else {
        8_000.0
    };
    assert!(
        ms < budget,
        "SH14 {loc} LOC rust compile {ms:.1} ms exceeds {budget}"
    );
}

/// SH14 large-project gate: 100k LOC in release (scaled down in debug so the suite stays usable).
#[test]
fn sh14_rust_compile_large_loc() {
    let n = if cfg!(debug_assertions) { 2_500 } else { 100_000 };
    let mut src = String::with_capacity(n * 12 + 24);
    src.push_str("let s = 0\n");
    for _ in 0..n {
        src.push_str("s = s + 1\n");
    }
    src.push_str("s\n");
    let loc = n + 2;
    let t0 = Instant::now();
    let p = kabootar_lib::compile::compile_source(&src).expect("large loc");
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    let loc_per_s = (loc as f64) / (ms / 1000.0).max(0.001);
    eprintln!("SH14 rust compile large loc={loc} ms={ms:.1} loc_per_s={loc_per_s:.0}");
    assert!(p.bytecode.is_some());
    // Debug 2.5k ~68 ms / ~37k loc/s; release 100k ~424 ms / ~236k loc/s after O(n) peephole.
    let budget = if cfg!(debug_assertions) {
        5_000.0
    } else {
        2_000.0
    };
    let min_loc_per_s = if cfg!(debug_assertions) {
        8_000.0
    } else {
        50_000.0
    };
    assert!(
        ms < budget,
        "SH14 {loc} LOC rust compile {ms:.1} ms exceeds {budget}"
    );
    assert!(
        loc_per_s >= min_loc_per_s,
        "SH14 {loc} LOC throughput {loc_per_s:.0} loc/s below {min_loc_per_s}"
    );
}

/// SH14 500k — release CI only (~2 s at measured loc/s). Debug stays on the 2.5k large gate.
#[test]
#[cfg(not(debug_assertions))]
fn sh14_rust_compile_500k_loc() {
    let n = 500_000;
    let mut src = String::with_capacity(n * 12 + 24);
    src.push_str("let s = 0\n");
    for _ in 0..n {
        src.push_str("s = s + 1\n");
    }
    src.push_str("s\n");
    let loc = n + 2;
    let t0 = Instant::now();
    let p = kabootar_lib::compile::compile_source(&src).expect("500k loc");
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    let loc_per_s = (loc as f64) / (ms / 1000.0).max(0.001);
    eprintln!("SH14 rust compile 500k loc={loc} ms={ms:.1} loc_per_s={loc_per_s:.0}");
    assert!(p.bytecode.is_some());
    assert!(
        ms < 10_000.0,
        "SH14 500k LOC rust compile {ms:.1} ms exceeds 10s"
    );
    assert!(
        loc_per_s >= 40_000.0,
        "SH14 500k throughput {loc_per_s:.0} loc/s below 40000"
    );
}

/// SH14 1M — release CI only (~5 s at measured loc/s). Debug stays on the 2.5k large gate.
#[test]
#[cfg(not(debug_assertions))]
fn sh14_rust_compile_1m_loc() {
    let n = 1_000_000;
    let mut src = String::with_capacity(n * 12 + 24);
    src.push_str("let s = 0\n");
    for _ in 0..n {
        src.push_str("s = s + 1\n");
    }
    src.push_str("s\n");
    let loc = n + 2;
    let t0 = Instant::now();
    let p = kabootar_lib::compile::compile_source(&src).expect("1m loc");
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    let loc_per_s = (loc as f64) / (ms / 1000.0).max(0.001);
    eprintln!("SH14 rust compile 1M loc={loc} ms={ms:.1} loc_per_s={loc_per_s:.0}");
    assert!(p.bytecode.is_some());
    assert!(
        ms < 10_000.0,
        "SH14 1M LOC rust compile {ms:.1} ms exceeds 10s"
    );
    assert!(
        loc_per_s >= 30_000.0,
        "SH14 1M throughput {loc_per_s:.0} loc/s below 30000"
    );
}
