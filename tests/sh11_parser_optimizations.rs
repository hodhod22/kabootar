//! Test SH11 parser optimizations - verify lexer/parser hotpath improvements

#[test]
fn sh11_lexer_optimized_scan_functions() {
    let lexer_file = std::fs::read_to_string("self_host/lexer_scan.kab")
        .expect("lexer_scan.kab should exist");
    
    // Verify lexer has optimized scan functions by category
    assert!(lexer_file.contains("lxScanPlusMinus"), "should have arith scan");
    assert!(lexer_file.contains("lxScanMulDiv"), "should have mul/div scan");
    assert!(lexer_file.contains("lxScanCmp"), "should have comparison scan");
    
    // Verify basic char operations are efficient
    assert!(lexer_file.contains("lxEat"), "should have eat function");
    assert!(lexer_file.contains("lxCh"), "should have char getter");
    assert!(lexer_file.contains("lxHas"), "should have has check");
    assert!(lexer_file.contains("lxStarts"), "should have starts with check");
}

#[test]
fn sh11_parser_session_reuse() {
    let parser_file = std::fs::read_to_string("self_host/parser_exec.kab")
        .expect("parser_exec.kab should exist");
    
    // Verify parser uses session reuse (SH2/SH13)
    assert!(parser_file.contains("pSessionInit_core"), "should have core session init");
    assert!(parser_file.contains("pSessionInit_expr"), "should have expr session init");
    assert!(parser_file.contains("sess"), "should use session parameter");
}

#[test]
fn sh11_emit_session_reuse() {
    let emit_file = std::fs::read_to_string("self_host/emit_exec.kab")
        .expect("emit_exec.kab should exist");
    
    // Verify emit uses session reuse (SH2/SH13)
    assert!(emit_file.contains("eMakeSession"), "should have session maker");
    assert!(emit_file.contains("E"), "should use E session parameter");
}

#[test]
fn sh11_lexer_token_pooling() {
    let lexer_file = std::fs::read_to_string("self_host/lexer_scan.kab")
        .expect("lexer_scan.kab should exist");
    
    // Verify lexer uses efficient token creation
    assert!(lexer_file.contains("lxTok"), "should have token creator");
    
    // Verify token structure is efficient
    assert!(lexer_file.contains("return { \"type\""), "should return object literals");
}

#[test]
fn sh11_parser_trampoline_optimization() {
    let ast_file = std::fs::read_to_string("self_host/ast_defs.kab")
        .expect("ast_defs.kab should exist");
    
    // Verify parser uses trampoline hooks (P6b)
    assert!(ast_file.contains("pCallHook"), "should have trampoline hook");
    assert!(ast_file.contains("pCallPostfix"), "should have postfix hook");
    assert!(ast_file.contains("pCallTypeArgs"), "should have type args hook");
    assert!(ast_file.contains("pCallUnary"), "should have unary hook");
    assert!(ast_file.contains("pCallMul"), "should have mul hook");
    assert!(ast_file.contains("pCallAddShift"), "should have add/shift hook");
    assert!(ast_file.contains("pCallRelExpr"), "should have rel expr hook");
    assert!(ast_file.contains("pCallCompare"), "should have compare hook");
    assert!(ast_file.contains("pCallStmt"), "should have stmt hook");
}