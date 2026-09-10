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
fn sh23_crypto_tls13_peer_appdata_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_peer_appdata.kab");
    assert!(file.exists(), "crypto_tls13_peer_appdata.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read appdata file");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_app\""), "should import app");
    assert!(content.contains("tls13PeerAppDataEvalOk"), "should have appdata eval");
    assert!(content.contains("tls13HsAppEvalOk"), "should call hs app eval");
}

#[test]
fn sh23_crypto_tls13_peer_x509_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_peer_x509.kab");
    assert!(file.exists(), "crypto_tls13_peer_x509.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read x509 file");
    
    assert!(content.contains("tls13PeerX509EvalOk"), "should have x509 eval");
    assert!(content.contains("tls13PeerX509Ok"), "should have x509 validator");
    assert!(content.contains("signatureAlgorithm"), "should validate signature alg");
    assert!(content.contains("signatureValue"), "should validate signature value");
}

#[test]
fn sh23_crypto_tls13_peer_x509_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_peer_x509_eval_smoke.kab");
    assert!(smoke.exists(), "x509 smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_peer_x509\""), "should import x509");
    assert!(content.contains("tls13PeerX509EvalOk"), "should call x509 eval");
    assert!(content.contains("28457"), "should use rustls port");
}

#[test]
fn sh23_crypto_tls13_client_fetch_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_client_fetch.kab");
    assert!(file.exists(), "crypto_tls13_client_fetch.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read client_fetch file");
    
    assert!(content.contains("tls13ClientFetch"), "should have client fetch");
    assert!(content.contains("httpBindTls13PeerFetch"), "should bind into http_fetch");
    assert!(content.contains("tls13ClientConnect"), "should connect");
    assert!(content.contains("tls13ClientGet"), "should get");
    assert!(content.contains("28296"), "should use TLS 1.3 peer port");
}

#[test]
fn sh23_crypto_tls13_client_fetch_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_client_fetch_eval_smoke.kab");
    assert!(smoke.exists(), "client fetch smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/http/http_fetch\""), "should import http_fetch");
    assert!(content.contains("import \"kab/crypto/crypto_tls13_client_fetch\""), "should import client fetch");
    assert!(content.contains("httpFetch"), "should call httpFetch");
    assert!(content.contains("28296"), "should use TLS 1.3 peer port");
}

#[test]
fn sh23_crypto_tls13_client_generic_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_client_generic.kab");
    assert!(file.exists(), "crypto_tls13_client_generic.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read client generic file");
    
    assert!(content.contains("tls13ClientGenericFetch"), "should have generic fetch");
    assert!(content.contains("tls13GenericUrlPort"), "should extract port");
    assert!(content.contains("tls13GenericHost"), "should extract host");
    assert!(content.contains("httpBindTls13GenericFetch"), "should bind into http_fetch");
}

#[test]
fn sh23_crypto_tls13_client_generic_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_client_generic_eval_smoke.kab");
    assert!(smoke.exists(), "client generic smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/http/http_fetch\""), "should import http_fetch");
    assert!(content.contains("import \"kab/crypto/crypto_tls13_client_generic\""), "should import generic");
    assert!(content.contains("httpFetch"), "should call httpFetch");
    assert!(content.contains("28296"), "should use TLS 1.3 peer port");
}

#[test]
fn sh23_crypto_tls13_client_timeout_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_client_timeout.kab");
    assert!(file.exists(), "crypto_tls13_client_timeout.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read client timeout file");
    
    assert!(content.contains("tls13ClientTimeoutEvalOk"), "should have timeout eval");
    assert!(content.contains("http_set_timeout"), "should set timeout");
    assert!(content.contains("http_reset_timeout"), "should reset timeout");
    assert!(content.contains("tls13ClientConnect"), "should connect");
}

#[test]
fn sh23_crypto_tls13_client_timeout_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_client_timeout_eval_smoke.kab");
    assert!(smoke.exists(), "client timeout smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_client_timeout\""), "should import client timeout");
    assert!(content.contains("tls13ClientTimeoutEvalOk"), "should call timeout eval");
}

#[test]
fn sh23_crypto_tls13_client_post_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_client_post.kab");
    assert!(file.exists(), "crypto_tls13_client_post.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read client post file");
    
    assert!(content.contains("tls13ClientPost"), "should have post");
    assert!(content.contains("httpBuildPost"), "should build POST request");
    assert!(content.contains("tls13PeerAppAfterHs"), "should send over TLS");
}

#[test]
fn sh23_crypto_tls13_client_post_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_client_post_eval_smoke.kab");
    assert!(smoke.exists(), "client post smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_client_post\""), "should import client post");
    assert!(content.contains("tls13ClientPost"), "should call post");
    assert!(content.contains("127.0.0.1"), "should use loopback");
    assert!(content.contains("28296"), "should use TLS 1.3 peer port");
}

#[test]
fn sh23_crypto_tls13_client_config_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_client_config.kab");
    assert!(file.exists(), "crypto_tls13_client_config.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read client config file");
    
    assert!(content.contains("tls13ClientConnectCfg"), "should have connect config");
    assert!(content.contains("tls13ClientGetCfg"), "should have get config");
    assert!(content.contains("tls13ClientConnect"), "should delegate to client");
    assert!(content.contains("cfg[\"port\"]"), "should use cfg port");
}

#[test]
fn sh23_crypto_tls13_client_config_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_client_config_eval_smoke.kab");
    assert!(smoke.exists(), "client config smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_client_config\""), "should import client config");
    assert!(content.contains("tls13ClientGetCfg"), "should call get config");
    assert!(content.contains("127.0.0.1"), "should use loopback");
    assert!(content.contains("28296"), "should use TLS 1.3 peer port");
}

#[test]
fn sh23_crypto_tls13_client_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_client.kab");
    assert!(file.exists(), "crypto_tls13_client.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read client file");
    
    assert!(content.contains("tls13ClientConnect"), "should have connect");
    assert!(content.contains("tls13ClientGet"), "should have get");
    assert!(content.contains("tls13PeerFullDerOk"), "should verify full handshake");
    assert!(content.contains("ecdsaP256Verify"), "should verify ECDSA");
}

#[test]
fn sh23_crypto_tls13_client_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_client_eval_smoke.kab");
    assert!(smoke.exists(), "client smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_client\""), "should import client");
    assert!(content.contains("tls13ClientConnect"), "should call connect");
    assert!(content.contains("\"127.0.0.1\""), "should use loopback host");
    assert!(content.contains("28296"), "should use TLS 1.3 HTTP peer port");
}

#[test]
fn sh23_crypto_tls13_peer_appdata_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_peer_appdata_eval_smoke.kab");
    assert!(smoke.exists(), "appdata smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_peer_appdata\""), "should import appdata");
    assert!(content.contains("tls13PeerAppDataEvalOk"), "should call appdata eval");
}

#[test]
fn sh23_crypto_tls13_peer_alert_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_peer_alert.kab");
    assert!(file.exists(), "crypto_tls13_peer_alert.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read alert file");
    
    assert!(content.contains("tls13PeerAlertEvalOk"), "should have alert eval");
    assert!(content.contains("tls13PeerHsClientOpen"), "should open handshake");
}

#[test]
fn sh23_crypto_tls13_peer_alert_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_peer_alert_eval_smoke.kab");
    assert!(smoke.exists(), "alert smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_peer_alert\""), "should import alert");
    assert!(content.contains("tls13PeerAlertEvalOk"), "should call alert eval");
    assert!(content.contains("28457"), "should use rustls port");
}

#[test]
fn sh23_crypto_tls13_peer_certreq_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_peer_certreq.kab");
    assert!(file.exists(), "crypto_tls13_peer_certreq.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read certreq file");
    
    assert!(content.contains("tls13PeerCertReqEvalOk"), "should have certreq eval");
    assert!(content.contains("s[\"cert\"]"), "should check cert");
    assert!(content.contains("s[\"cv\"]"), "should check cv");
}

#[test]
fn sh23_crypto_tls13_peer_certreq_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_peer_certreq_eval_smoke.kab");
    assert!(smoke.exists(), "certreq smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_peer_certreq\""), "should import certreq");
    assert!(content.contains("tls13PeerCertReqEvalOk"), "should call certreq eval");
    assert!(content.contains("28457"), "should use rustls port");
}

#[test]
fn sh23_crypto_tls13_peer_resume_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_peer_resume.kab");
    assert!(file.exists(), "crypto_tls13_peer_resume.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read resume file");
    
    assert!(content.contains("tls13PeerResumeEvalOk"), "should have resume eval");
    assert!(content.contains("s[\"cert\"]"), "should check cert");
}

#[test]
fn sh23_crypto_tls13_peer_resume_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_peer_resume_eval_smoke.kab");
    assert!(smoke.exists(), "resume smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_peer_resume\""), "should import resume");
    assert!(content.contains("tls13PeerResumeEvalOk"), "should call resume eval");
    assert!(content.contains("28457"), "should use rustls port");
}

#[test]
fn sh23_crypto_tls13_peer_crl_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_peer_crl.kab");
    assert!(file.exists(), "crypto_tls13_peer_crl.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read crl file");
    
    assert!(content.contains("tls13PeerCrlEvalOk"), "should have crl eval");
    assert!(content.contains("tls13PeerCrldpOk"), "should check crldp");
    assert!(content.contains("tls13PeerCrldpOid"), "should check crldp OID");
    assert!(content.contains("2.5.29.31"), "should know CRLDP OID");
}

#[test]
fn sh23_crypto_tls13_peer_crl_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_peer_crl_eval_smoke.kab");
    assert!(smoke.exists(), "crl smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_peer_crl\""), "should import crl");
    assert!(content.contains("tls13PeerCrlEvalOk"), "should call crl eval");
    assert!(content.contains("28457"), "should use rustls port");
}

#[test]
fn sh23_crypto_tls13_peer_ocsp_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_peer_ocsp.kab");
    assert!(file.exists(), "crypto_tls13_peer_ocsp.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read ocsp file");
    
    assert!(content.contains("tls13PeerOcspEvalOk"), "should have ocsp eval");
    assert!(content.contains("tls13PeerAiaOk"), "should validate AIA");
    assert!(content.contains("1.3.6.1.5.5.7.48.1"), "should check OCSP method");
}

#[test]
fn sh23_crypto_tls13_peer_ocsp_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_peer_ocsp_eval_smoke.kab");
    assert!(smoke.exists(), "ocsp smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_peer_ocsp\""), "should import ocsp");
    assert!(content.contains("tls13PeerOcspEvalOk"), "should call ocsp eval");
}

#[test]
fn sh23_crypto_tls13_peer_chainall_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_peer_chainall.kab");
    assert!(file.exists(), "crypto_tls13_peer_chainall.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read chainall file");
    
    assert!(content.contains("tls13PeerChainAllEvalOk"), "should have chainall eval");
    assert!(content.contains("tls13PeerChainCertOk"), "should validate each cert");
    assert!(content.contains("tls13PeerChainDerAt"), "should walk cert list");
}

#[test]
fn sh23_crypto_tls13_peer_chainall_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_peer_chainall_eval_smoke.kab");
    assert!(smoke.exists(), "chainall smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_peer_chainall\""), "should import chainall");
    assert!(content.contains("tls13PeerChainAllEvalOk"), "should call chainall eval");
    assert!(content.contains("28457"), "should use rustls port");
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
fn sh23_crypto_tls13_client_loop_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_client_loop.kab");
    assert!(file.exists(), "crypto_tls13_client_loop.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read client_loop file");
    
    assert!(content.contains("tls13ClientNoRustlsEvalOk"), "should have no-rustls eval");
    assert!(content.contains("tls13HsAppEvalOk"), "should call loopback app eval");
    assert!(content.contains("crypto_tls13_hs"), "should import handshake");
}

#[test]
fn sh23_crypto_tls13_client_loop_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_client_loop_eval_smoke.kab");
    assert!(smoke.exists(), "client_loop smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_client_loop\""), "should import client_loop");
    assert!(content.contains("tls13ClientNoRustlsEvalOk"), "should call no-rustls eval");
}

#[test]
fn sh23_crypto_tls13_client_close_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_client_close.kab");
    assert!(file.exists(), "crypto_tls13_client_close.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read client close file");
    
    assert!(content.contains("tls13ClientCloseEvalOk"), "should have close eval");
    assert!(content.contains("tls13ClientCloseNotify"), "should build close_notify");
    assert!(content.contains("tcp_close"), "should close socket");
    assert!(content.contains("tcp_write_bytes"), "should write alert");
}

#[test]
fn sh23_crypto_tls13_client_close_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_client_close_eval_smoke.kab");
    assert!(smoke.exists(), "client close smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_client_close\""), "should import client close");
    assert!(content.contains("tls13ClientCloseEvalOk"), "should call close eval");
    assert!(content.contains("28457"), "should use rustls port");
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
fn sh23_crypto_tls13_peer_trust_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_peer_trust.kab");
    assert!(file.exists(), "crypto_tls13_peer_trust.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read trust file");
    
    assert!(content.contains("tls13PeerTrustEvalOk"), "should have trust eval");
    assert!(content.contains("tls13PeerTrustDerOk"), "should have trust der check");
    assert!(content.contains("tls13PeerTbsParts"), "should parse TBS parts");
    assert!(content.contains("cryptoRootDerEqual"), "should check issuer==subject");
}

#[test]
fn sh23_crypto_tls13_peer_full_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_peer_full.kab");
    assert!(file.exists(), "crypto_tls13_peer_full.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read full file");
    
    assert!(content.contains("tls13PeerFullEvalOk"), "should have full eval");
    assert!(content.contains("tls13PeerChainAllOk"), "should check chain structure");
    assert!(content.contains("tls13PeerTrustDerOk"), "should check leaf trust");
    assert!(content.contains("ecdsaP256Verify"), "should verify ECDSA signature");
    assert!(content.contains("tls13PeerCvSignedHash"), "should compute CV signed hash");
}

#[test]
fn sh23_crypto_tls13_peer_full_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_peer_full_eval_smoke.kab");
    assert!(smoke.exists(), "full smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_peer_full\""), "should import full");
    assert!(content.contains("tls13PeerFullEvalOk"), "should call full eval");
    assert!(content.contains("28457"), "should use rustls port");
}

#[test]
fn sh23_crypto_tls13_peer_trust_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_peer_trust_eval_smoke.kab");
    assert!(smoke.exists(), "trust smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_peer_trust\""), "should import trust");
    assert!(content.contains("tls13PeerTrustEvalOk"), "should call trust eval");
    assert!(content.contains("28457"), "should use rustls port");
}

#[test]
fn sh23_crypto_tls13_peer_scheme_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_peer_scheme.kab");
    assert!(file.exists(), "crypto_tls13_peer_scheme.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read scheme file");
    
    assert!(content.contains("tls13PeerSchemeEvalOk"), "should have scheme eval");
    assert!(content.contains("tls13PeerSchemeSpkiOk"), "should check SPKI");
    assert!(content.contains("ecdsa_secp256r1_sha256"), "should check scheme 0x0403");
}

#[test]
fn sh23_crypto_tls13_peer_scheme_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_peer_scheme_eval_smoke.kab");
    assert!(smoke.exists(), "scheme smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_peer_scheme\""), "should import scheme");
    assert!(content.contains("tls13PeerSchemeEvalOk"), "should call scheme eval");
    assert!(content.contains("28457"), "should use rustls port");
}

#[test]
fn sh23_crypto_tls13_client_get_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_client_get.kab");
    assert!(file.exists(), "crypto_tls13_client_get.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read client_get file");
    
    assert!(content.contains("tls13ClientGetOk"), "should have client_get eval");
    assert!(content.contains("tls13ClientConnect"), "should call client connect");
    assert!(content.contains("tls13ClientGet"), "should call client get");
}

#[test]
fn sh23_crypto_tls13_client_reconnect_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_client_reconnect.kab");
    assert!(file.exists(), "crypto_tls13_client_reconnect.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read reconnect file");
    
    assert!(content.contains("tls13ClientReconnectEvalOk"), "should have reconnect eval");
    assert!(content.contains("tls13ClientConnect"), "should call client connect");
    assert!(content.contains("tcp_close"), "should close sockets");
    assert!(content.contains("28296"), "should use TLS 1.3 HTTP peer port");
}

#[test]
fn sh23_crypto_tls13_client_reconnect_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_client_reconnect_eval_smoke.kab");
    assert!(smoke.exists(), "reconnect smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_client_reconnect\""), "should import reconnect");
    assert!(content.contains("tls13ClientReconnectEvalOk"), "should call reconnect eval");
}

#[test]
fn sh23_crypto_tls13_all_exists() {
    let file = std::path::Path::new("lib/kab/crypto/crypto_tls13_all.kab");
    assert!(file.exists(), "crypto_tls13_all.kab should exist");
    
    let content = std::fs::read_to_string(file)
        .expect("should read all file");
    
    assert!(content.contains("cryptoTls13AllOk"), "should have all eval");
    assert!(content.contains("tls13ClientGetOk"), "should test client GET");
    assert!(content.contains("tls13ClientReconnectEvalOk"), "should test reconnect");
    assert!(content.contains("tls13ClientNoRustlsEvalOk"), "should test no-rustls");
    assert!(content.contains("tls13ClientTimeoutEvalOk"), "should test timeout");
}

#[test]
fn sh23_crypto_tls13_all_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_all_eval_smoke.kab");
    assert!(smoke.exists(), "all smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_all\""), "should import all");
    assert!(content.contains("cryptoTls13AllOk"), "should call all eval");
}

#[test]
fn sh23_crypto_tls13_client_get_smoke_exists() {
    let smoke = std::path::Path::new("examples/sh23_crypto_tls13_client_get_eval_smoke.kab");
    assert!(smoke.exists(), "client_get smoke should exist");
    
    let content = std::fs::read_to_string(smoke)
        .expect("should read smoke");
    
    assert!(content.contains("import \"kab/crypto/crypto_tls13_client_get\""), "should import client_get");
    assert!(content.contains("tls13ClientGetOk"), "should call client_get eval");
}

#[test]
fn sh23_crypto_tls13_host_delete_policy() {
    let crypto_host_file = std::fs::read_to_string("lib/kab/crypto/crypto_host.kab")
        .expect("crypto_host.kab should exist");
    
    // Verify delete gate delegates to the aggregate SH23 smoke gate
    assert!(crypto_host_file.contains("cryptoHostDeleteOk"), "should have delete ok function");
    assert!(crypto_host_file.contains("cryptoTls13AllOk"), "should use aggregate gate");
    
    // Make sure it's not accidentally hardcoded to true/false
    assert!(!crypto_host_file.contains("return true"), "delete gate should not be hardcoded true");
    assert!(!crypto_host_file.contains("return false"), "delete gate should not be hardcoded false");
}