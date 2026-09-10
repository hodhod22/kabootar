//! Test SH23 crypto TLS 1.3 peer validation - verify SAN length 12 rejection and generalized validator

#[test]
fn sh23_crypto_tls13_peer_n12_exists() {
    let n12_file = std::path::Path::new("lib/kab/crypto/crypto_tls13_peer_n12.kab");
    assert!(n12_file.exists(), "crypto_tls13_peer_n12.kab should exist");
    
    let n12_content = std::fs::read_to_string(n12_file)
        .expect("should read n12 file");
    
    // Verify n12 file is based on n11
    assert!(n12_content.contains("import \"kab/crypto/crypto_tls13_peer_n11\""), "should import n11");
    assert!(n12_content.contains("tls13PeerSanN12Loop"), "should have n12 loop function");
    assert!(n12_content.contains("tls13PeerN12EvalOk"), "should have n12 eval function");
    
    // Verify n12 rejects length 12
    assert!(n12_content.contains("if name[\"len\"] == 12"), "should reject length 12");
}

#[test]
fn sh23_crypto_tls13_peer_n12_smoke_exists() {
    let smoke_file = std::path::Path::new("examples/sh23_crypto_tls13_peer_n12_eval_smoke.kab");
    assert!(smoke_file.exists(), "n12 smoke test should exist");
    
    let smoke_content = std::fs::read_to_string(smoke_file)
        .expect("should read smoke file");
    
    assert!(smoke_content.contains("import \"kab/crypto/crypto_tls13_peer_n12\""), "should import n12");
    assert!(smoke_content.contains("tls13PeerN12EvalOk"), "should call n12 eval");
    assert!(smoke_content.contains("28457"), "should use rustls port 28457");
}

#[test]
fn sh23_crypto_tls13_peer_n13_exists() {
    let n13_file = std::path::Path::new("lib/kab/crypto/crypto_tls13_peer_n13.kab");
    assert!(n13_file.exists(), "crypto_tls13_peer_n13.kab should exist");
    
    let n13_content = std::fs::read_to_string(n13_file)
        .expect("should read n13 file");
    
    // Verify n13 file uses the generalized SAN validator
    assert!(n13_content.contains("import \"kab/crypto/crypto_tls13_peer_san_gen\""), "should import san_gen");
    assert!(n13_content.contains("tls13PeerSanN13Loop"), "should have n13 loop function");
    assert!(n13_content.contains("tls13PeerN13EvalOk"), "should have n13 eval function");
    assert!(n13_content.contains("tls13PeerSanValidateIp4"), "should use generalized ip4 validator");
    
    // Verify n13 rejects length 13
    assert!(n13_content.contains("if name[\"len\"] == 13"), "should reject length 13");
}

#[test]
fn sh23_crypto_tls13_peer_n13_smoke_exists() {
    let smoke_file = std::path::Path::new("examples/sh23_crypto_tls13_peer_n13_eval_smoke.kab");
    assert!(smoke_file.exists(), "n13 smoke test should exist");
    
    let smoke_content = std::fs::read_to_string(smoke_file)
        .expect("should read smoke file");
    
    assert!(smoke_content.contains("import \"kab/crypto/crypto_tls13_peer_n13\""), "should import n13");
    assert!(smoke_content.contains("tls13PeerN13EvalOk"), "should call n13 eval");
    assert!(smoke_content.contains("28457"), "should use rustls port 28457");
}

#[test]
fn sh23_crypto_tls13_peer_n14_exists() {
    let n14_file = std::path::Path::new("lib/kab/crypto/crypto_tls13_peer_n14.kab");
    assert!(n14_file.exists(), "crypto_tls13_peer_n14.kab should exist");
    
    let n14_content = std::fs::read_to_string(n14_file)
        .expect("should read n14 file");
    
    // Verify n14 file uses the generalized SAN validator
    assert!(n14_content.contains("import \"kab/crypto/crypto_tls13_peer_san_gen\""), "should import san_gen");
    assert!(n14_content.contains("tls13PeerSanN14Loop"), "should have n14 loop function");
    assert!(n14_content.contains("tls13PeerN14EvalOk"), "should have n14 eval function");
    assert!(n14_content.contains("tls13PeerSanValidateIp4"), "should use generalized ip4 validator");
    
    // Verify n14 rejects length 14
    assert!(n14_content.contains("if name[\"len\"] == 14"), "should reject length 14");
}

#[test]
fn sh23_crypto_tls13_peer_n14_smoke_exists() {
    let smoke_file = std::path::Path::new("examples/sh23_crypto_tls13_peer_n14_eval_smoke.kab");
    assert!(smoke_file.exists(), "n14 smoke test should exist");
    
    let smoke_content = std::fs::read_to_string(smoke_file)
        .expect("should read smoke file");
    
    assert!(smoke_content.contains("import \"kab/crypto/crypto_tls13_peer_n14\""), "should import n14");
    assert!(smoke_content.contains("tls13PeerN14EvalOk"), "should call n14 eval");
    assert!(smoke_content.contains("28457"), "should use rustls port 28457");
}

#[test]
fn sh23_crypto_tls13_peer_n11_exists() {
    let n11_file = std::path::Path::new("lib/kab/crypto/crypto_tls13_peer_n11.kab");
    assert!(n11_file.exists(), "crypto_tls13_peer_n11.kab should exist");
    
    let n11_content = std::fs::read_to_string(n11_file)
        .expect("should read n11 file");
    
    // Verify n11 file structure
    assert!(n11_content.contains("tls13PeerSanN11Loop"), "should have n11 loop function");
    assert!(n11_content.contains("tls13PeerN11EvalOk"), "should have n11 eval function");
    
    // Verify n11 rejects length 11
    assert!(n11_content.contains("if name[\"len\"] == 11"), "should reject length 11");
}

#[test]
fn sh23_crypto_tls13_san_gen_exists() {
    let san_gen_file = std::path::Path::new("lib/kab/crypto/crypto_tls13_peer_san_gen.kab");
    assert!(san_gen_file.exists(), "crypto_tls13_peer_san_gen.kab should exist");
    
    let san_gen_content = std::fs::read_to_string(san_gen_file)
        .expect("should read san_gen file");
    
    // Verify generalized SAN validator
    assert!(san_gen_content.contains("tls13PeerSanValidateIp4"), "should have ip4 validator");
    assert!(san_gen_content.contains("tls13PeerSanValidateLoop"), "should have san validate loop");
    assert!(san_gen_content.contains("tls13PeerSanValidateEvalOk"), "should have eval function");
    
    // Verify it imports existing san module
    assert!(san_gen_content.contains("import \"kab/crypto/crypto_tls13_peer_san\""), "should import san module");
    
    // Verify it rejects all non-iPAddress tags
    assert!(san_gen_content.contains("if name[\"tag\"] != 135"), "should reject non-iPAddress tags");
    
    // Verify it checks for 127.0.0.1
    assert!(san_gen_content.contains("!= 127"), "should check 127 first octet");
    assert!(san_gen_content.contains("== 1"), "should check last octet = 1");
}

#[test]
fn sh23_crypto_tls13_san_gen_smoke_exists() {
    let smoke_file = std::path::Path::new("examples/sh23_crypto_tls13_peer_san_gen_eval_smoke.kab");
    assert!(smoke_file.exists(), "san_gen smoke test should exist");
    
    let smoke_content = std::fs::read_to_string(smoke_file)
        .expect("should read smoke file");
    
    assert!(smoke_content.contains("import \"kab/crypto/crypto_tls13_peer_san_gen\""), "should import san_gen");
    assert!(smoke_content.contains("tls13PeerSanValidateEvalOk"), "should call san_gen eval");
    assert!(smoke_content.contains("28457"), "should use rustls port 28457");
}

#[test]
fn sh23_crypto_tls13_peer_n15_exists() {
    let n15_file = std::path::Path::new("lib/kab/crypto/crypto_tls13_peer_n15.kab");
    assert!(n15_file.exists(), "crypto_tls13_peer_n15.kab should exist");
    
    let n15_content = std::fs::read_to_string(n15_file)
        .expect("should read n15 file");
    
    assert!(n15_content.contains("import \"kab/crypto/crypto_tls13_peer_san_gen\""), "should import san_gen");
    assert!(n15_content.contains("tls13PeerSanN15Loop"), "should have n15 loop function");
    assert!(n15_content.contains("tls13PeerN15EvalOk"), "should have n15 eval function");
    assert!(n15_content.contains("tls13PeerSanValidateIp4"), "should use generalized ip4 validator");
    assert!(n15_content.contains("if name[\"len\"] == 15"), "should reject length 15");
}

#[test]
fn sh23_crypto_tls13_peer_n15_smoke_exists() {
    let smoke_file = std::path::Path::new("examples/sh23_crypto_tls13_peer_n15_eval_smoke.kab");
    assert!(smoke_file.exists(), "n15 smoke test should exist");
    
    let smoke_content = std::fs::read_to_string(smoke_file)
        .expect("should read smoke file");
    
    assert!(smoke_content.contains("import \"kab/crypto/crypto_tls13_peer_n15\""), "should import n15");
    assert!(smoke_content.contains("tls13PeerN15EvalOk"), "should call n15 eval");
    assert!(smoke_content.contains("28457"), "should use rustls port 28457");
}

#[test]
fn sh23_crypto_tls13_peer_n4_exists() {
    let n4_file = std::path::Path::new("lib/kab/crypto/crypto_tls13_peer_n4.kab");
    assert!(n4_file.exists(), "crypto_tls13_peer_n4.kab should exist");
    
    let n4_content = std::fs::read_to_string(n4_file)
        .expect("should read n4 file");
    
    assert!(n4_content.contains("import \"kab/crypto/crypto_tls13_peer_san_gen\""), "should import san_gen");
    assert!(n4_content.contains("tls13PeerSanN4Loop"), "should have n4 loop function");
    assert!(n4_content.contains("tls13PeerN4EvalOk"), "should have n4 eval function");
    assert!(n4_content.contains("tls13PeerSanValidateLoop"), "should use generalized san validate loop");
}

#[test]
fn sh23_crypto_tls13_peer_n4_smoke_exists() {
    let smoke_file = std::path::Path::new("examples/sh23_crypto_tls13_peer_n4_eval_smoke.kab");
    assert!(smoke_file.exists(), "n4 smoke test should exist");
    
    let smoke_content = std::fs::read_to_string(smoke_file)
        .expect("should read smoke file");
    
    assert!(smoke_content.contains("import \"kab/crypto/crypto_tls13_peer_n4\""), "should import n4");
    assert!(smoke_content.contains("tls13PeerN4EvalOk"), "should call n4 eval");
    assert!(smoke_content.contains("28457"), "should use rustls port 28457");
}

#[test]
fn sh23_crypto_tls13_peer_dates_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_peer_dates.kab");
    assert!(file.exists(), "crypto_tls13_peer_dates.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read dates file");
    
    assert!(content.contains("tls13PeerValidityEvalOk"), "should have validity eval");
    assert!(content.contains("tls13PeerTimeParse"), "should parse time");
    assert!(content.contains("UTCTime"), "should handle UTCTime");
    assert!(content.contains("notBefore"), "should check notBefore");
}

#[test]
fn sh23_crypto_tls13_peer_sigalg_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_peer_sigalg.kab");
    assert!(file.exists(), "crypto_tls13_peer_sigalg.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read sigalg file");
    
    assert!(content.contains("tls13PeerSigAlgEvalOk"), "should have sigalg eval");
    assert!(content.contains("tls13PeerSigAlgOk"), "should have sigalg validator");
    assert!(content.contains("outerAlg"), "should check outer signature algorithm");
    assert!(content.contains("innerAlg"), "should check inner signature algorithm");
}

#[test]
fn sh23_crypto_tls13_peer_chain_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_peer_chain.kab");
    assert!(file.exists(), "crypto_tls13_peer_chain.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read chain file");
    
    assert!(content.contains("tls13PeerChainCount"), "should count chain");
    assert!(content.contains("tls13PeerChainEvalOk"), "should have chain eval");
    assert!(content.contains("tls13PeerChainLenOk"), "should have chain length check");
}

#[test]
fn sh23_crypto_tls13_peer_chain_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_peer_chain_eval_smoke.kab");
    assert!(smoke.exists(), "chain smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_peer_chain\""), "should import chain");
    assert!(content.contains("tls13PeerChainEvalOk"), "should call chain eval");
    assert!(content.contains("28457"), "should use rustls port");
}

#[test]
fn sh23_crypto_tls13_peer_sigalg_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_peer_sigalg_eval_smoke.kab");
    assert!(smoke.exists(), "sigalg smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_peer_sigalg\""), "should import sigalg");
    assert!(content.contains("tls13PeerSigAlgEvalOk"), "should call sigalg eval");
    assert!(content.contains("28457"), "should use rustls port");
}

#[test]
fn sh23_crypto_tls13_peer_dates_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_peer_dates_eval_smoke.kab");
    assert!(smoke.exists(), "dates smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_peer_dates\""), "should import dates");
    assert!(content.contains("tls13PeerValidityEvalOk"), "should call validity eval");
    assert!(content.contains("28457"), "should use rustls port 28457");
}

#[test]
fn sh23_crypto_tls13_peer_serial_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_peer_serial.kab");
    assert!(file.exists(), "crypto_tls13_peer_serial.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read serial file");
    
    assert!(content.contains("tls13PeerSerialEvalOk"), "should have serial eval");
    assert!(content.contains("tls13PeerSerialOk"), "should have serial validator");
    assert!(content.contains("serial[\"len\"]"), "should check serial length");
}

#[test]
fn sh23_crypto_tls13_peer_pin_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_peer_pin.kab");
    assert!(file.exists(), "crypto_tls13_peer_pin.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read pin file");
    
    assert!(content.contains("tls13PeerPinEvalOk"), "should have pin eval");
    assert!(content.contains("tls13PeerPinEqual"), "should have pin equality");
    assert!(content.contains("tcp_close"), "should close sockets");
}

#[test]
fn sh23_crypto_tls13_peer_pin_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_peer_pin_eval_smoke.kab");
    assert!(smoke.exists(), "pin smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_peer_pin\""), "should import pin");
    assert!(content.contains("tls13PeerPinEvalOk"), "should call pin eval");
    assert!(content.contains("28457"), "should use rustls port");
}

#[test]
fn sh23_crypto_tls13_peer_serial_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_peer_serial_eval_smoke.kab");
    assert!(smoke.exists(), "serial smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_peer_serial\""), "should import serial");
    assert!(content.contains("tls13PeerSerialEvalOk"), "should call serial eval");
    assert!(content.contains("28457"), "should use rustls port");
}

#[test]
fn sh23_crypto_tls13_basic_peer() {
    let peer_file = std::path::Path::new("lib/kab/crypto/crypto_tls13_peer.kab");
    assert!(peer_file.exists(), "crypto_tls13_peer.kab should exist");
    
    let peer_content = std::fs::read_to_string(peer_file)
        .expect("should read peer file");
    
    // Verify basic peer implementation
    assert!(peer_content.contains("tls13PeerBuildClientHello"), "should have client hello builder");
    assert!(peer_content.contains("tls13PeerHelloEvalOk"), "should have hello eval function");
    assert!(peer_content.contains("tcp_connect"), "should use tcp connect");
    assert!(peer_content.contains("tls13ParseServerHello"), "should parse server hello");
}

#[test]
fn sh23_crypto_tls13_hs_opt_exists() {
    let hs_opt_file = std::path::Path::new("lib/kab/crypto/crypto_tls13_hs_opt.kab");
    assert!(hs_opt_file.exists(), "crypto_tls13_hs_opt.kab should exist");
    
    let hs_opt_content = std::fs::read_to_string(hs_opt_file)
        .expect("should read hs_opt file");
    
    // Verify consolidated extension scanner
    assert!(hs_opt_content.contains("tls13ScanExtOpt"), "should have shared scanner");
    assert!(hs_opt_content.contains("tls13ScanChExtOpt"), "should have client scanner");
    assert!(hs_opt_content.contains("tls13ScanShExtOpt"), "should have server scanner");
    assert!(hs_opt_content.contains("flags"), "should use parameterized flags");
}

#[test]
fn sh23_crypto_tls13_hs_opt_smoke_exists() {
    let smoke_file = std::path::Path::new("examples/sh23_crypto_tls13_hs_opt_smoke.kab");
    assert!(smoke_file.exists(), "hs_opt smoke test should exist");
    
    let smoke_content = std::fs::read_to_string(smoke_file)
        .expect("should read smoke file");
    
    assert!(smoke_content.contains("import \"kab/crypto/crypto_tls13_hs_opt\""), "should import hs_opt");
}

#[test]
fn sh23_crypto_tls13_peer_nlen_exists() {
    let nlen_file = std::path::Path::new("lib/kab/crypto/crypto_tls13_peer_nlen.kab");
    assert!(nlen_file.exists(), "crypto_tls13_peer_nlen.kab should exist");
    
    let nlen_content = std::fs::read_to_string(nlen_file)
        .expect("should read nlen file");
    
    // Verify parameterized length validator
    assert!(nlen_content.contains("pub fn tls13PeerSanNLenLoop"), "should have nlen loop");
    assert!(nlen_content.contains("pub fn tls13PeerNLenEvalOk"), "should have nlen eval");
    assert!(nlen_content.contains("badLen"), "should have badLen parameter");
    assert!(nlen_content.contains("tls13PeerSanValidateIp4"), "should use generalized ip4 validator");
    
    // Verify it imports existing san module
    assert!(nlen_content.contains("import \"kab/crypto/crypto_tls13_peer_san_gen\""), "should import san_gen");
    assert!(nlen_content.contains("import \"kab/crypto/crypto_tls13_peer_san\""), "should import san");
}

#[test]
fn sh23_crypto_tls13_peer_nlen_sweep_smoke_exists() {
    let smoke_file = std::path::Path::new("examples/sh23_crypto_tls13_peer_nlen_sweep_smoke.kab");
    assert!(smoke_file.exists(), "nlen sweep smoke test should exist");
    
    let smoke_content = std::fs::read_to_string(smoke_file)
        .expect("should read sweep smoke file");
    
    assert!(smoke_content.contains("import \"kab/crypto/crypto_tls13_peer_nlen\""), "should import nlen");
    assert!(smoke_content.contains("tls13PeerNLenEvalOk"), "should call nlen eval");
    assert!(smoke_content.contains("allBadLengthsRejected"), "should sweep bad lengths");
    assert!(smoke_content.contains("[0, 1, 2, 3"), "should cover multiple lengths");
}

#[test]
fn sh23_crypto_tls13_peer_nlen_smoke_exists() {
    let smoke_file = std::path::Path::new("examples/sh23_crypto_tls13_peer_nlen_eval_smoke.kab");
    assert!(smoke_file.exists(), "nlen smoke test should exist");
    
    let smoke_content = std::fs::read_to_string(smoke_file)
        .expect("should read smoke file");
    
    assert!(smoke_content.contains("import \"kab/crypto/crypto_tls13_peer_nlen\""), "should import nlen");
    assert!(smoke_content.contains("tls13PeerNLenEvalOk"), "should call nlen eval");
    assert!(smoke_content.contains("28457"), "should use rustls port 28457");
    assert!(smoke_content.contains("11"), "should use badLen 11");
}

#[test]
fn sh23_crypto_tls13_host_delete_policy() {
    let crypto_host_file = std::fs::read_to_string("lib/kab/crypto/crypto_host.kab")
        .expect("crypto_host.kab should exist");
    
    // Verify delete gate is correctly closed (false) until smoke-complete
    assert!(crypto_host_file.contains("cryptoHostDeleteOk"), "should have delete ok function");
    assert!(crypto_host_file.contains("return false"), "delete gate should be false");
    
    // Make sure it's not accidentally set to true
    assert!(!crypto_host_file.contains("return true"), "delete gate should not be true yet");
}