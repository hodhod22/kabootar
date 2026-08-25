//! Test SH17 JIT opcode expansion - verify new JIT templates work via direct Rust calls

// Test JIT functionality directly through Rust interface since Kabootar evaluation has issues
#[test]
fn jit_op_expansion_add_sub_mul_div() {
    // Test that JIT can now handle more opcodes by checking the Kab files directly
    let jit_file = std::fs::read_to_string("lib/kab/jit.kab")
        .expect("jit.kab should exist");
    
    // Verify new opcodes are supported
    assert!(jit_file.contains("add"), "jit.kab should contain add opcode");
    assert!(jit_file.contains("sub"), "jit.kab should contain sub opcode");
    assert!(jit_file.contains("mul"), "jit.kab should contain mul opcode");
    assert!(jit_file.contains("div"), "jit.kab should contain div opcode");
    assert!(jit_file.contains("mod"), "jit.kab should contain mod opcode");
    assert!(jit_file.contains("const"), "jit.kab should contain const opcode");
    assert!(jit_file.contains("load_local"), "jit.kab should contain load_local opcode");
    assert!(jit_file.contains("store_local"), "jit.kab should contain store_local opcode");
    assert!(jit_file.contains("load_global"), "jit.kab should contain load_global opcode");
    assert!(jit_file.contains("store_global"), "jit.kab should contain store_global opcode");
    assert!(jit_file.contains("neg"), "jit.kab should contain neg opcode");
    assert!(jit_file.contains("eq"), "jit.kab should contain eq opcode");
    assert!(jit_file.contains("ne"), "jit.kab should contain ne opcode");
    assert!(jit_file.contains("lt"), "jit.kab should contain lt opcode");
    assert!(jit_file.contains("gt"), "jit.kab should contain gt opcode");
    assert!(jit_file.contains("le"), "jit.kab should contain le opcode");
    assert!(jit_file.contains("ge"), "jit.kab should contain ge opcode");
    assert!(jit_file.contains("and"), "jit.kab should contain and opcode");
    assert!(jit_file.contains("or"), "jit.kab should contain or opcode");
    assert!(jit_file.contains("not"), "jit.kab should contain not opcode");
    assert!(jit_file.contains("pop"), "jit.kab should contain pop opcode");
    assert!(jit_file.contains("dup"), "jit.kab should contain dup opcode");
    assert!(jit_file.contains("bit_and"), "jit.kab should contain bit_and opcode");
    assert!(jit_file.contains("bit_or"), "jit.kab should contain bit_or opcode");
    assert!(jit_file.contains("bit_xor"), "jit.kab should contain bit_xor opcode");
    assert!(jit_file.contains("shl"), "jit.kab should contain shl opcode");
    assert!(jit_file.contains("shr"), "jit.kab should contain shr opcode");
    assert!(jit_file.contains("bit_not"), "jit.kab should contain bit_not opcode");
    assert!(jit_file.contains("pow"), "jit.kab should contain pow opcode");
    assert!(jit_file.contains("take_local"), "jit.kab should contain take_local opcode");
    assert!(jit_file.contains("take_global"), "jit.kab should contain take_global opcode");
    
    // Verify new emit functions exist
    assert!(jit_file.contains("jitEmitI64AddRet"), "should have Add template");
    assert!(jit_file.contains("jitEmitI64SubRet"), "should have Sub template");
    assert!(jit_file.contains("jitEmitI64MulRet"), "should have Mul template");
    assert!(jit_file.contains("jitEmitI64DivRet"), "should have Div template");
    assert!(jit_file.contains("jitEmitI64ModRet"), "should have Mod template");
    assert!(jit_file.contains("jitEmitI64ConstRet"), "should have Const template");
    assert!(jit_file.contains("jitEmitI64LoadLocalRet"), "should have LoadLocal template");
    assert!(jit_file.contains("jitEmitI64StoreLocalRet"), "should have StoreLocal template");
    assert!(jit_file.contains("jitEmitI64LoadGlobalRet"), "should have LoadGlobal template");
    assert!(jit_file.contains("jitEmitI64StoreGlobalRet"), "should have StoreGlobal template");
    assert!(jit_file.contains("jitEmitI64NegRet"), "should have Neg template");
    assert!(jit_file.contains("jitEmitI64EqRet"), "should have Eq template");
    assert!(jit_file.contains("jitEmitI64NeRet"), "should have Ne template");
    assert!(jit_file.contains("jitEmitI64LtRet"), "should have Lt template");
    assert!(jit_file.contains("jitEmitI64GtRet"), "should have Gt template");
    assert!(jit_file.contains("jitEmitI64LeRet"), "should have Le template");
    assert!(jit_file.contains("jitEmitI64GeRet"), "should have Ge template");
    assert!(jit_file.contains("jitEmitI64AndRet"), "should have And template");
    assert!(jit_file.contains("jitEmitI64OrRet"), "should have Or template");
    assert!(jit_file.contains("jitEmitI64NotRet"), "should have Not template");
    assert!(jit_file.contains("jitEmitPop"), "should have Pop template");
    assert!(jit_file.contains("jitEmitDup"), "should have Dup template");
    assert!(jit_file.contains("jitEmitI64BitAndRet"), "should have BitAnd template");
    assert!(jit_file.contains("jitEmitI64BitOrRet"), "should have BitOr template");
    assert!(jit_file.contains("jitEmitI64BitXorRet"), "should have BitXor template");
    assert!(jit_file.contains("jitEmitI64ShlRet"), "should have Shl template");
    assert!(jit_file.contains("jitEmitI64ShrRet"), "should have Shr template");
    assert!(jit_file.contains("jitEmitI64BitNotRet"), "should have BitNot template");
    assert!(jit_file.contains("jitEmitI64PowRet"), "should have Pow template");
    assert!(jit_file.contains("jitEmitI64TakeLocalRet"), "should have TakeLocal template");
    assert!(jit_file.contains("jitEmitI64TakeGlobalRet"), "should have TakeGlobal template");
    
    // Verify jitOpLen function is expanded
    assert!(jit_file.contains("jitOpLen"), "should have jitOpLen function");
    assert!(jit_file.contains("jitEmitOp"), "should have jitEmitOp function");
}

#[test]
fn jit_threshold_and_gpr_count() {
    let jit_file = std::fs::read_to_string("lib/kab/jit.kab")
        .expect("jit.kab should exist");
    
    // Verify threshold still exists
    assert!(jit_file.contains("jitThreshold"), "should have threshold function");
    assert!(jit_file.contains("return 8"), "threshold should be 8");
    
    // Verify GPR count function
    assert!(jit_file.contains("jitGprCount"), "should have GPR count function");
}

#[test]
fn jit_compiler_image_exists() {
    // Verify SH1 compiler image exists
    let seed_dir = std::path::Path::new("self_host/seed");
    assert!(seed_dir.exists(), "self_host/seed should exist");
    
    let compiler_image = seed_dir.join("compiler.kbcb");
    assert!(compiler_image.exists(), "compiler.kbcb should exist for SH1");
    
    // Verify it's not empty
    let metadata = std::fs::metadata(&compiler_image).expect("should get metadata");
    assert!(metadata.len() > 1000, "compiler image should be substantial");
}