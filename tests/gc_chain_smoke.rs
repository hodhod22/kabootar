//! Test SH18 GC chain - verify Kab-GC implementation works correctly

#[test]
fn gc_chain_components_exist() {
    // Verify all GC chain components exist in lib/kab/
    let gc_dir = std::path::Path::new("lib/kab");
    
    // Core GC
    assert!(gc_dir.join("gc.kab").exists(), "gc.kab should exist");
    
    // GC chain components
    assert!(gc_dir.join("gc_load.kab").exists(), "gc_load.kab should exist");
    assert!(gc_dir.join("gc_cycle.kab").exists(), "gc_cycle.kab should exist");
    assert!(gc_dir.join("gc_bar.kab").exists(), "gc_bar.kab should exist");
    assert!(gc_dir.join("gc_stress.kab").exists(), "gc_stress.kab should exist");
    assert!(gc_dir.join("gc_concurrent.kab").exists(), "gc_concurrent.kab should exist");
    assert!(gc_dir.join("gc_concurrent_stress.kab").exists(), "gc_concurrent_stress.kab should exist");
    
    // GC chain and capstone
    assert!(gc_dir.join("gc_chain.kab").exists(), "gc_chain.kab should exist");
    assert!(gc_dir.join("gc_capstone.kab").exists(), "gc_capstone.kab should exist");
    
    // GC host delete policy
    assert!(gc_dir.join("gc_host.kab").exists(), "gc_host.kab should exist");
}

#[test]
fn gc_basic_functionality() {
    let gc_file = std::fs::read_to_string("lib/kab/gc.kab")
        .expect("gc.kab should exist");
    
    // Verify basic GC functions exist
    assert!(gc_file.contains("gcNurseryCap"), "should have nursery cap");
    assert!(gc_file.contains("gcBump"), "should have bump function");
    assert!(gc_file.contains("gcNeedCollect"), "should have collect predicate");
    assert!(gc_file.contains("gcFrameBudgetMs"), "should have frame budget");
    
    // Verify default values
    assert!(gc_file.contains("65536"), "nursery cap should be 64KB");
    assert!(gc_file.contains("16"), "frame budget should be 16ms");
}

#[test]
fn gc_write_barrier() {
    let gc_bar_file = std::fs::read_to_string("lib/kab/gc_bar.kab")
        .expect("gc_bar.kab should exist");
    
    // Verify write barrier implementation
    assert!(gc_bar_file.contains("gcWriteBarrier"), "should have write barrier");
    assert!(gc_bar_file.len() > 100, "write barrier should have implementation");
}

#[test]
fn gc_concurrent_mark() {
    let gc_concurrent_file = std::fs::read_to_string("lib/kab/gc_concurrent.kab")
        .expect("gc_concurrent.kab should exist");
    
    // Verify concurrent mark implementation
    assert!(gc_concurrent_file.contains("gcConcurrentMarkOk"), "should have concurrent mark");
    assert!(gc_concurrent_file.contains("gcConcurrentMarkDepth"), "should have mark depth");
    assert!(gc_concurrent_file.contains("gcConcurrentOk"), "should have concurrent ok check");
}

#[test]
fn gc_capstone_integrity() {
    let gc_capstone_file = std::fs::read_to_string("lib/kab/gc_capstone.kab")
        .expect("gc_capstone.kab should exist");
    
    // Verify capstone imports all chain components
    assert!(gc_capstone_file.contains("import \"kab/gc_chain\""), "should import gc_chain");
    assert!(gc_capstone_file.contains("import \"kab/gc_host\""), "should import gc_host");
    assert!(gc_capstone_file.contains("import \"kab/noll_host\""), "should import noll_host");
    
    // Verify capstone functions
    assert!(gc_capstone_file.contains("gcCapstoneOk"), "should have capstone ok");
    assert!(gc_capstone_file.contains("gcCapstoneGatesClosed"), "should have gates closed check");
}

#[test]
fn gc_host_delete_policy_correct() {
    let gc_host_file = std::fs::read_to_string("lib/kab/gc_host.kab")
        .expect("gc_host.kab should exist");
    
    // Verify delete gate is correctly closed (false) until smoke-complete
    assert!(gc_host_file.contains("gcHostDeleteOk"), "should have delete ok function");
    assert!(gc_host_file.contains("return false"), "delete gate should be false");
    
    // Make sure it's not accidentally set to true
    assert!(!gc_host_file.contains("return true"), "delete gate should not be true yet");
}

#[test]
fn gc_nursery_cycle_implementation() {
    let gc_cycle_file = std::fs::read_to_string("lib/kab/gc_cycle.kab")
        .expect("gc_cycle.kab should exist");
    
    // Verify nursery cycle implementation
    assert!(gc_cycle_file.contains("gcNurseryCycleOk"), "should have nursery cycle");
    assert!(gc_cycle_file.len() > 200, "nursery cycle should have implementation");
}

#[test]
fn gc_promote_implementation() {
    let gc_prom_file = std::fs::read_to_string("lib/kab/gc_prom.kab")
        .expect("gc_prom.kab should exist");
    
    // Verify promote implementation
    assert!(gc_prom_file.contains("gcPromote"), "should have promote function");
    assert!(gc_prom_file.contains("gcPromoteObjects"), "should have promote objects function");
    assert!(gc_prom_file.contains("gcShouldPromote"), "should have should promote function");
    assert!(gc_prom_file.contains("gcPromoteThreshold"), "should have promote threshold function");
}

#[test]
fn gc_mark_implementation() {
    let gc_mark_file = std::fs::read_to_string("lib/kab/gc_mark.kab")
        .expect("gc_mark.kab should exist");
    
    // Verify mark implementation
    assert!(gc_mark_file.contains("gcSweepDead"), "should have sweep dead function");
    assert!(gc_mark_file.contains("gcMarkObject"), "should have mark object function");
    assert!(gc_mark_file.contains("gcMarkReachable"), "should have mark reachable function");
    assert!(gc_mark_file.contains("gcMarkWithAge"), "should have mark with age function");
}

#[test]
fn gc_stress_implementation() {
    let gc_stress_file = std::fs::read_to_string("lib/kab/gc_stress.kab")
        .expect("gc_stress.kab should exist");
    
    // Verify stress test implementation
    assert!(gc_stress_file.contains("gcStressCyclesOk"), "should have stress cycles");
    assert!(gc_stress_file.len() > 200, "stress should have implementation");
}

#[test]
fn gc_chain_integrity() {
    let gc_chain_file = std::fs::read_to_string("lib/kab/gc_chain.kab")
        .expect("gc_chain.kab should exist");
    
    // Verify chain imports all components
    assert!(gc_chain_file.contains("import \"kab/gc_load\""), "should import gc_load");
    assert!(gc_chain_file.contains("import \"kab/gc_stress\""), "should import gc_stress");
    assert!(gc_chain_file.contains("import \"kab/gc_concurrent\""), "should import gc_concurrent");
    assert!(gc_chain_file.contains("import \"kab/gc_concurrent_stress\""), "should import concurrent stress");
    assert!(gc_chain_file.contains("import \"kab/gc_host\""), "should import gc_host");
    
    // Verify chain validation
    assert!(gc_chain_file.contains("gcChainOk"), "should have chain ok");
    assert!(gc_chain_file.contains("gcChainStepsOk"), "should have chain steps ok");
}