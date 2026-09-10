//! Test SH6 Kab-VM optimize - tag-family fast lookup

#[test]
fn sh6_vm_tag_array_exists() {
    let file = std::path::Path::new("self_host/vm_run_tag_array.kab");
    assert!(file.exists(), "vm_run_tag_array.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read tag_array file");
    
    assert!(content.contains("VM_TAG_FAM"), "should define tag-family array");
    assert!(content.contains("vmTagFamily"), "should have family lookup");
    assert!(content.contains("return -1"), "should return -1 for invalid tag");
}

#[test]
fn sh6_vm_tag_array_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh6_vm_tag_array_smoke.kab");
    assert!(smoke.exists(), "sh6_vm_tag_array_smoke.kab should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"self_host/vm_run_tag_array\""), "should import tag_array");
    assert!(content.contains("vmTagFamily"), "should call family lookup");
}

#[test]
fn sh6_vm_dispatch_plain_exists() {
    let file = std::path::Path::new("self_host/vm_run_dispatch_plain.kab");
    assert!(file.exists(), "vm_run_dispatch_plain.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read dispatch file");
    
    assert!(content.contains("runOpPlainS"), "should have plain dispatcher");
    assert!(content.contains("runOpPlainMacd"), "should have macd dispatcher");
}

#[test]
fn sh6_vm_ops_loop_exists() {
    let file = std::path::Path::new("self_host/vm_run_ops_loop.kab");
    assert!(file.exists(), "vm_run_ops_loop.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read ops loop");
    
    assert!(content.contains("runOpTaggedS"), "should use tagged dispatch");
    assert!(content.contains("runOpsS"), "should have main loop");
    assert!(content.contains("KBCB_OP_FAM"), "should have op family string");
}