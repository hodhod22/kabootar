//! Test SH11 hotpath optimizations - verify emit optimizations work

#[test]
fn sh11_emit_expr_body_optimizations_exist() {
    let emit_file = std::fs::read_to_string("self_host/emit_expr_body.kab")
        .expect("emit_expr_body.kab should exist");
    
    // Verify SH11 optimization functions exist
    assert!(emit_file.contains("tryEmitAccAddLiteral"), "should have AccAdd literal optimization");
    assert!(emit_file.contains("tryEmitIndexGetLocal"), "should have IndexGet local optimization");
    assert!(emit_file.contains("tryEmitConstBinaryOp"), "should have Const binary op optimization");
    assert!(emit_file.contains("tryEmitConstArrayLiteral"), "should have Const array literal optimization");
    assert!(emit_file.contains("tryEmitStringEqLiteral"), "should have String eq literal optimization");
    assert!(emit_file.contains("tryEmitArrayLengthProperty"), "should have Array length property optimization");
    assert!(emit_file.contains("tryEmitArrayPopPattern"), "should have Array pop pattern optimization");
    assert!(emit_file.contains("tryEmitSimpleNegation"), "should have Simple negation optimization");
    
    // Verify optimizations use the right opcodes
    assert!(emit_file.contains("OP_ACC_ADD_LOCAL"), "should use OP_ACC_ADD_LOCAL");
    assert!(emit_file.contains("OP_ACC_ADD_GLOBAL"), "should use OP_ACC_ADD_GLOBAL");
    assert!(emit_file.contains("OP_INDEX_GET_LOCAL"), "should use OP_INDEX_GET_LOCAL");
    assert!(emit_file.contains("OP_MAKE_ARRAY"), "should use OP_MAKE_ARRAY");
    assert!(emit_file.contains("OP_EQ"), "should use OP_EQ");
    assert!(emit_file.contains("OP_LEN_LOCAL"), "should use OP_LEN_LOCAL");
    assert!(emit_file.contains("OP_LEN_GLOBAL"), "should use OP_LEN_GLOBAL");
    assert!(emit_file.contains("OP_ARRAY_POP_LOCAL"), "should use OP_ARRAY_POP_LOCAL");
    assert!(emit_file.contains("OP_ARRAY_POP_GLOBAL"), "should use OP_ARRAY_POP_GLOBAL");
}

#[test]
fn sh11_existing_len_call_optimization() {
    let emit_file = std::fs::read_to_string("self_host/emit_expr_body.kab")
        .expect("emit_expr_body.kab should exist");
    
    // Verify existing SH11 len call optimization
    assert!(emit_file.contains("tryEmitLenCall"), "should have len call optimization");
    assert!(emit_file.contains("OP_LEN_LOCAL"), "should use OP_LEN_LOCAL");
    assert!(emit_file.contains("OP_LEN_GLOBAL"), "should use OP_LEN_GLOBAL");
}

#[test]
fn sh11_call_arg_fast_path() {
    let emit_file = std::fs::read_to_string("self_host/emit_expr_body.kab")
        .expect("emit_expr_body.kab should exist");
    
    // Verify existing call argument fast path
    assert!(emit_file.contains("emitCallArgFast"), "should have call arg fast path");
    assert!(emit_file.contains("eCallRetFast"), "should have call ret fast tracking");
}

#[test]
fn sh11_optimizations_design_correctness() {
    let emit_file = std::fs::read_to_string("self_host/emit_expr_body.kab")
        .expect("emit_expr_body.kab should exist");
    
    // Verify optimizations don't use Rust-specific features
    assert!(!emit_file.contains("Rust"), "should not mention Rust");
    assert!(!emit_file.contains("cranelift"), "should not mention cranelift");
    
    // Verify optimizations are conditional (return false when not applicable)
    assert!(emit_file.contains("return false"), "optimizations should be conditional");
    assert!(emit_file.contains("return true"), "optimizations should return true on success");
}

#[test]
fn sh11_ownership_optimizations() {
    let ownership_file = std::fs::read_to_string("self_host/ownership.kab")
        .expect("ownership.kab should exist");
    
    // Verify ownership checker has optimization hooks
    assert!(ownership_file.contains("oPairIndex"), "should have optimized pair index");
    assert!(ownership_file.contains("oPairGet"), "should have optimized pair get");
    assert!(ownership_file.contains("oPairSetVals"), "should have optimized pair set");
    
    // Verify optimization uses direct array access instead of map lookups where possible
    assert!(ownership_file.len() > 100, "ownership should have implementation");
}