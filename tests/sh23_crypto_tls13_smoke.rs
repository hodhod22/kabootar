//! Test SH23 crypto TLS 1.3 peer validation - verify SAN length 12 rejection

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
fn sh23_crypto_tls13_host_delete_policy() {
    let crypto_host_file = std::fs::read_to_string("lib/kab/crypto/crypto_host.kab")
        .expect("crypto_host.kab should exist");
    
    // Verify delete gate is correctly closed (false) until smoke-complete
    assert!(crypto_host_file.contains("cryptoHostDeleteOk"), "should have delete ok function");
    assert!(crypto_host_file.contains("return false"), "delete gate should be false");
    
    // Make sure it's not accidentally set to true
    assert!(!crypto_host_file.contains("return true"), "delete gate should not be true yet");
}