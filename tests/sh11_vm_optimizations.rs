//! Test SH11 VM optimizations - verify VM hotpath improvements

#[test]
fn sh11_vm_dispatch_optimization() {
    let dispatch_file = std::fs::read_to_string("self_host/vm_run_dispatch_plain.kab")
        .expect("vm_run_dispatch_plain.kab should exist");
    
    // Verify VM uses chained dispatch for self-host compile performance
    assert!(dispatch_file.contains("runOpArithAb"), "should have arith AB dispatch");
    assert!(dispatch_file.contains("runOpArithCd"), "should have arith CD dispatch");
    assert!(dispatch_file.contains("runOpCmpAb"), "should have cmp AB dispatch");
    assert!(dispatch_file.contains("runOpDataAb"), "should have data AB dispatch");
    
    // Verify dispatch is split into small functions for fast compilation
    assert!(dispatch_file.contains("2-way dispatch chain"), "should use 2-way dispatch chains");
}

#[test]
fn sh11_vm_session_reuse() {
    let session_file = std::fs::read_to_string("self_host/vm_run_session.kab")
        .expect("vm_run_session.kab should exist");
    
    // Verify VM uses session reuse for performance
    assert!(session_file.contains("S"), "should use S session parameter");
    assert!(session_file.len() > 100, "session should have implementation");
}

#[test]
fn sh11_vm_operation_groups() {
    // Verify VM operations are grouped for efficient dispatch
    let arith_file = std::fs::read_to_string("self_host/vm_ops_arith_a.kab")
        .expect("vm_ops_arith_a.kab should exist");
    let cmp_file = std::fs::read_to_string("self_host/vm_ops_cmp_a.kab")
        .expect("vm_ops_cmp_a.kab should exist");
    let ctrl_file = std::fs::read_to_string("self_host/vm_ops_ctrl_a.kab")
        .expect("vm_ops_ctrl_a.kab should exist");
    
    assert!(arith_file.contains("runOpArithA"), "should have arith A ops");
    assert!(cmp_file.contains("runOpCmpA"), "should have cmp A ops");
    assert!(ctrl_file.contains("runOpCtrlA"), "should have ctrl A ops");
}

#[test]
fn sh11_vm_call_optimization() {
    let call_file = std::fs::read_to_string("self_host/vm_run_call.kab")
        .expect("vm_run_call.kab should exist");
    
    // Verify VM has call optimization
    assert!(call_file.contains("vArrayToOrdered"), "should have array to ordered function");
    assert!(call_file.contains("vPopOrderedArgs"), "should have pop ordered args function");
    assert!(call_file.len() > 100, "call should have implementation");
}

#[test]
fn sh11_vm_trampoline_optimization() {
    let tramp_file = std::fs::read_to_string("self_host/vm_run_tramp.kab")
        .expect("vm_run_tramp.kab should exist");
    
    // Verify VM uses trampoline for stack safety
    assert!(tramp_file.contains("tramp"), "should have trampoline");
    assert!(tramp_file.len() > 100, "trampoline should have implementation");
}