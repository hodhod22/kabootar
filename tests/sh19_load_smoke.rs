//! Test SH19 loader deepen - packed .kbcb header validation

#[test]
fn sh19_load_validate_exists() {
    let file = std::path::Path::new("lib/kab/load/load_validate.kab");
    assert!(file.exists(), "load_validate.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read load_validate");
    
    assert!(content.contains("loadKbcbHeaderOk"), "should have header validator");
    assert!(content.contains("loadKbcbFixtureOk"), "should have fixture validator");
    assert!(content.contains("loadKbcbMagicOk"), "should use magic validator");
    assert!(content.contains("version"), "should check version");
    assert!(content.contains("bodyLen"), "should check body length");
}

#[test]
fn sh19_load_validate_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh19_load_validate_smoke.kab");
    assert!(smoke.exists(), "sh19_load_validate_smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/load/load_validate\""), "should import load_validate");
    assert!(content.contains("loadKbcbFixtureOk"), "should call fixture validator");
}

#[test]
fn sh19_load_main_delete_policy() {
    let main = std::fs::read_to_string("lib/kab/load/load_main.kab")
        .expect("load_main.kab should exist");
    
    assert!(main.contains("loadMainDeleteOk"), "should have delete policy");
    assert!(main.contains("return false"), "loadMainDeleteOk should be false");
    assert!(!main.contains("return true"), "loadMainDeleteOk should not be true");
}

#[test]
fn sh19_load_kbcb_exists() {
    let file = std::path::Path::new("lib/kab/load/load_kbcb.kab");
    assert!(file.exists(), "load_kbcb.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read load_kbcb");
    
    assert!(content.contains("loadEvalKbcbRoundtripOk"), "should have roundtrip eval");
    assert!(content.contains("loadKbcbMagicOk"), "should have magic check");
    assert!(content.contains("loadEvalKbcbFile"), "should have file eval");
}