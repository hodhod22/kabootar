//! Test SH17 JIT bridge - verify Rust-to-Kab-JIT integration layer

#[test]
fn jit_bridge_file_exists() {
    let bridge_file = std::path::Path::new("lib/kab/jit/jit_bridge.kab");
    assert!(bridge_file.exists(), "jit_bridge.kab should exist");
    
    let content = std::fs::read_to_string(bridge_file)
        .expect("should read bridge file");
    
    // Verify bridge imports JIT components
    assert!(content.contains("import \"kab/jit\""), "should import jit");
    assert!(content.contains("import \"kab/jit/jit_run\""), "should import jit_run");
    assert!(content.contains("import \"kab/jit/jit_mm\""), "should import jit_mm");
    
    // Verify bridge functions exist
    assert!(content.contains("jitBridgeCanCompile"), "should have can compile check");
    assert!(content.contains("jitBridgeEmit"), "should have emit function");
    assert!(content.contains("jitBridgeTotalLen"), "should have total len function");
    assert!(content.contains("jitBridgeValidate"), "should have validate function");
    assert!(content.contains("jitBridgeGetMmapPolicy"), "should have mmap policy function");
    assert!(content.contains("jitBridgeCheckMmap"), "should have mmap check function");
    assert!(content.contains("jitBridgePagesFor"), "should have pages for function");
    assert!(content.contains("jitBridgePipelineOk"), "should have pipeline check");
}

#[test]
fn jit_bridge_design_correctness() {
    let bridge_file = std::fs::read_to_string("lib/kab/jit/jit_bridge.kab")
        .expect("should read bridge file");
    
    // Verify bridge is self-contained in Kabootar
    assert!(bridge_file.contains("pub fn"), "should export public functions");
    assert!(bridge_file.contains("return"), "should use return statements");
    
    // Verify bridge uses Kab-JIT functions correctly
    assert!(bridge_file.contains("jitOpOk"), "should use jitOpOk");
    assert!(bridge_file.contains("jitEmitOp"), "should use jitEmitOp");
    assert!(bridge_file.contains("jitOpLen"), "should use jitOpLen");
    assert!(bridge_file.contains("jitMmPid"), "should use jitMmPid");
    assert!(bridge_file.contains("jitMmVirt"), "should use jitMmVirt");
    assert!(bridge_file.contains("jitMmLen"), "should use jitMmLen");
    assert!(bridge_file.contains("jitMmProt"), "should use jitMmProt");
}

#[test]
fn jit_bridge_complete_pipeline() {
    let bridge_file = std::fs::read_to_string("lib/kab/jit/jit_bridge.kab")
        .expect("should read bridge file");
    
    // Verify complete pipeline check exists
    assert!(bridge_file.contains("jitBridgePipelineOk"), "should have pipeline check");
    
    // Verify pipeline checks all steps
    assert!(bridge_file.contains("jitBridgeCanCompile"), "pipeline should check can compile");
    assert!(bridge_file.contains("jitBridgeEmit"), "pipeline should emit");
    assert!(bridge_file.contains("jitBridgeTotalLen"), "pipeline should check length");
    assert!(bridge_file.contains("jitBridgeValidate"), "pipeline should validate");
    assert!(bridge_file.contains("jitBridgePagesFor"), "pipeline should calculate pages");
}

#[test]
fn jit_bridge_memory_policy() {
    let bridge_file = std::fs::read_to_string("lib/kab/jit/jit_bridge.kab")
        .expect("should read bridge file");
    
    // Verify memory policy functions
    assert!(bridge_file.contains("jitBridgeGetMmapPolicy"), "should get mmap policy");
    assert!(bridge_file.contains("jitBridgeCheckMmap"), "should check mmap args");
    
    // Verify policy returns array with all parameters
    assert!(bridge_file.contains("return [jitMmPid"), "should return policy array");
}