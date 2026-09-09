//! SH23: TLS 1.2 CKE/Finished vs rustls peer (test-only rustls, not product src).

#![cfg(not(target_arch = "wasm32"))]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::time::Duration;

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes128Gcm, Nonce};
use kabootar_lib::evaluator::create_global_env;
use rustls::KeyLog;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use num_bigint::BigUint;
use num_traits::{One, Zero};
use sha2::{Digest, Sha256};

const ALICE_SECRET: [u8; 32] = [
    0x77, 0x07, 0x6d, 0x0a, 0x73, 0x18, 0xa5, 0x7d, 0x3c, 0x16, 0xc1, 0x72, 0x51, 0xb2, 0x66, 0x45,
    0xdf, 0x4c, 0x2f, 0x87, 0xeb, 0xc0, 0x99, 0x2a, 0xb1, 0x77, 0xfb, 0xa5, 0x1d, 0xb9, 0x2c, 0x2a,
];
const ALICE_PUB: [u8; 32] = [
    0x85, 0x20, 0xf0, 0x09, 0x89, 0x30, 0xa7, 0x54, 0x74, 0x8b, 0x7d, 0xdc, 0xb4, 0x3e, 0xf7, 0x5a,
    0x0d, 0xbf, 0x3a, 0x0d, 0x26, 0x38, 0x1a, 0xf4, 0xeb, 0xa4, 0xa9, 0x8e, 0xaa, 0x9b, 0x4e, 0x6a,
];

fn x25519_rfc7748(k: [u8; 32], mut u: [u8; 32]) -> [u8; 32] {
    let mut k = k;
    k[0] &= 248;
    k[31] &= 127;
    k[31] |= 64;
    u[31] &= 127;
    let p: BigUint = (BigUint::one() << 255) - BigUint::from(19u32);
    let a24 = BigUint::from(121665u32);
    let x1 = BigUint::from_bytes_le(&u);
    let mut x2 = BigUint::one();
    let mut z2 = BigUint::zero();
    let mut x3 = x1.clone();
    let mut z3 = BigUint::one();
    let mut swap = false;
    for t in (0..=254).rev() {
        let kt = ((k[t / 8] >> (t % 8)) & 1) == 1;
        swap ^= kt;
        if swap {
            std::mem::swap(&mut x2, &mut x3);
            std::mem::swap(&mut z2, &mut z3);
        }
        swap = kt;
        let a = (&x2 + &z2) % &p;
        let aa = a.clone().modpow(&BigUint::from(2u32), &p);
        let b = (&p + &x2 - &z2) % &p;
        let bb = b.clone().modpow(&BigUint::from(2u32), &p);
        let e = (&p + &aa - &bb) % &p;
        let c = (&x3 + &z3) % &p;
        let d = (&p + &x3 - &z3) % &p;
        let da = (d * a.clone()) % &p;
        let cb = (c * b) % &p;
        let sum = (&da + &cb) % &p;
        let dif = (&p + &da - &cb) % &p;
        x3 = sum.clone().modpow(&BigUint::from(2u32), &p);
        z3 = (&x1 * dif.clone().modpow(&BigUint::from(2u32), &p)) % &p;
        x2 = (&aa * &bb) % &p;
        z2 = (&e * ((&aa + a24.clone() * e.clone()) % &p)) % &p;
    }
    if swap {
        std::mem::swap(&mut x2, &mut x3);
        std::mem::swap(&mut z2, &mut z3);
    }
    let zinv = z2.modpow(&(&p - 2u32), &p);
    let out = (x2 * zinv) % p;
    let mut bytes = out.to_bytes_le();
    bytes.resize(32, 0);
    bytes.try_into().unwrap()
}

#[derive(Debug)]
struct PrintKeyLog;

impl KeyLog for PrintKeyLog {
    fn log(&self, label: &str, client_random: &[u8], secret: &[u8]) {
        eprint!("keylog {label} cr=");
        for b in client_random {
            eprint!("{b:02x}");
        }
        eprint!(" secret=");
        for b in secret {
            eprint!("{b:02x}");
        }
        eprintln!();
    }
}

fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut kpad = [0u8; 64];
    if key.len() > 64 {
        let d = Sha256::digest(key);
        kpad[..32].copy_from_slice(&d);
    } else {
        kpad[..key.len()].copy_from_slice(key);
    }
    let mut ipad = kpad;
    let mut opad = kpad;
    for b in ipad.iter_mut() {
        *b ^= 0x36;
    }
    for b in opad.iter_mut() {
        *b ^= 0x5c;
    }
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(msg);
    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner.finalize());
    outer.finalize().into()
}

fn tls_prf(secret: &[u8], label: &[u8], seed: &[u8], n: usize) -> Vec<u8> {
    let mut seed_all = Vec::with_capacity(label.len() + seed.len());
    seed_all.extend_from_slice(label);
    seed_all.extend_from_slice(seed);
    let mut a = hmac_sha256(secret, &seed_all);
    let mut out = Vec::new();
    while out.len() < n {
        let mut p = Vec::with_capacity(32 + seed_all.len());
        p.extend_from_slice(&a);
        p.extend_from_slice(&seed_all);
        out.extend_from_slice(&hmac_sha256(secret, &p));
        a = hmac_sha256(secret, &a);
    }
    out.truncate(n);
    out
}

fn read_exact(tcp: &mut TcpStream, n: usize) -> Vec<u8> {
    let mut buf = vec![0u8; n];
    tcp.read_exact(&mut buf).expect("read");
    buf
}

fn read_record(tcp: &mut TcpStream) -> (u8, Vec<u8>) {
    let head = read_exact(tcp, 5);
    let typ = head[0];
    let size = u16::from_be_bytes([head[3], head[4]]) as usize;
    (typ, read_exact(tcp, size))
}

fn take_hs(buf: &mut Vec<u8>) -> Option<Vec<u8>> {
    if buf.len() < 4 {
        return None;
    }
    let len = ((buf[1] as usize) << 16) | ((buf[2] as usize) << 8) | buf[3] as usize;
    let total = 4 + len;
    if buf.len() < total {
        return None;
    }
    Some(buf.drain(..total).collect())
}

fn spawn_tls12_peer(port: u16, keylog: bool, serve_http: bool) {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let cert = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_string()]).unwrap();
    let cert_der = CertificateDer::from(cert.cert.der().to_vec());
    let key_der = PrivateKeyDer::Pkcs8(cert.key_pair.serialize_der().into());
    let mut server_cfg =
        rustls::ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS12])
            .with_no_client_auth()
            .with_single_cert(vec![cert_der], key_der)
            .unwrap();
    server_cfg.send_tls13_tickets = 0;
    server_cfg.require_ems = false;
    if keylog {
        server_cfg.key_log = Arc::new(PrintKeyLog);
    }
    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).expect("bind");
    let server_cfg = Arc::new(server_cfg);
    std::thread::spawn(move || {
        let (mut tcp, _) = match listener.accept() {
            Ok(p) => p,
            Err(_) => return,
        };
        let _ = tcp.set_read_timeout(Some(Duration::from_secs(1800)));
        let mut conn = rustls::ServerConnection::new(server_cfg).unwrap();
        loop {
            while conn.wants_write() {
                if conn.write_tls(&mut tcp).is_err() {
                    return;
                }
            }
            if !conn.is_handshaking() {
                break;
            }
            match conn.read_tls(&mut tcp) {
                Ok(0) => return,
                Ok(_) => {
                    if let Err(e) = conn.process_new_packets() {
                        eprintln!("rustls process_new_packets: {e}");
                        return;
                    }
                }
                Err(_) => return,
            }
        }
        while conn.wants_write() {
            let _ = conn.write_tls(&mut tcp);
        }
        eprintln!("rustls handshake complete");
        if !serve_http {
            return;
        }
        loop {
            while conn.wants_write() {
                if conn.write_tls(&mut tcp).is_err() {
                    return;
                }
            }
            let mut buf = [0u8; 4096];
            match conn.reader().read(&mut buf) {
                Ok(0) => return,
                Ok(n) => {
                    if buf[..n].starts_with(b"GET ")
                        || buf[..n].starts_with(b"POST ")
                        || buf[..n].starts_with(b"PUT ")
                        || buf[..n].starts_with(b"PATCH ")
                        || buf[..n].starts_with(b"DELETE ")
                        || buf[..n].starts_with(b"HEAD ")
                        || buf[..n].starts_with(b"OPTIONS ")
                        || buf[..n].starts_with(b"TRACE ")
                        || buf[..n].starts_with(b"CONNECT ")
                    {
                        let _ = conn.writer().write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK",
                        );
                        let _ = conn.writer().flush();
                        while conn.wants_write() {
                            let _ = conn.write_tls(&mut tcp);
                        }
                    }
                    return;
                }
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    match conn.read_tls(&mut tcp) {
                        Ok(0) => return,
                        Ok(_) => {
                            if let Err(e) = conn.process_new_packets() {
                                eprintln!("rustls process_new_packets: {e}");
                                return;
                            }
                        }
                        Err(_) => return,
                    }
                }
                Err(_) => return,
            }
        }
    });
}

fn spawn_tls13_rsa_peer(port: u16) {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut rng = rand::thread_rng();
    let privk = rsa::RsaPrivateKey::new(&mut rng, 2048).expect("rsa 2048");
    let pkcs8 = rsa::pkcs8::EncodePrivateKey::to_pkcs8_der(&privk).expect("pkcs8");
    let pkcs8_der = rustls::pki_types::PrivatePkcs8KeyDer::from(pkcs8.as_bytes().to_vec());
    let key_pair = rcgen::KeyPair::from_pkcs8_der_and_sign_algo(
        &pkcs8_der,
        &rcgen::PKCS_RSA_SHA256,
    )
    .expect("rcgen rsa key");
    let params = rcgen::CertificateParams::new(vec!["127.0.0.1".to_string()]).unwrap();
    let cert = params.self_signed(&key_pair).unwrap();
    let cert_der = CertificateDer::from(cert.der().to_vec());
    let key_der = PrivateKeyDer::Pkcs8(key_pair.serialize_der().into());
    let mut server_cfg =
        rustls::ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
            .with_no_client_auth()
            .with_single_cert(vec![cert_der], key_der)
            .unwrap();
    server_cfg.send_tls13_tickets = 0;
    server_cfg.key_log = Arc::new(PrintKeyLog);
    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).expect("bind rsa peer");
    let server_cfg = Arc::new(server_cfg);
    std::thread::spawn(move || {
        let (mut tcp, _) = match listener.accept() {
            Ok(p) => p,
            Err(_) => return,
        };
        let _ = tcp.set_read_timeout(Some(Duration::from_secs(1800)));
        let mut conn = rustls::ServerConnection::new(server_cfg).unwrap();
        loop {
            while conn.wants_write() {
                if conn.write_tls(&mut tcp).is_err() {
                    return;
                }
            }
            if !conn.is_handshaking() {
                break;
            }
            match conn.read_tls(&mut tcp) {
                Ok(0) => return,
                Ok(_) => {
                    if let Err(e) = conn.process_new_packets() {
                        eprintln!("rustls TLS 1.3 RSA process_new_packets: {e}");
                        return;
                    }
                }
                Err(_) => return,
            }
        }
        while conn.wants_write() {
            let _ = conn.write_tls(&mut tcp);
        }
        eprintln!("rustls TLS 1.3 RSA handshake complete");
    });
}

fn spawn_tls13_peer(port: u16, keylog: bool, serve_http: bool) {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut params = rcgen::CertificateParams::new(vec!["127.0.0.1".to_string()]).unwrap();
    params.key_usages = vec![rcgen::KeyUsagePurpose::DigitalSignature];
    params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ServerAuth];
    params.is_ca = rcgen::IsCa::ExplicitNoCa;
    params.use_authority_key_identifier_extension = true;
    let key_pair = rcgen::KeyPair::generate().unwrap();
    let cert = params.self_signed(&key_pair).unwrap();
    let cert_der = CertificateDer::from(cert.der().to_vec());
    let key_der = PrivateKeyDer::Pkcs8(key_pair.serialize_der().into());
    let mut server_cfg =
        rustls::ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])
            .with_no_client_auth()
            .with_single_cert(vec![cert_der], key_der)
            .unwrap();
    server_cfg.send_tls13_tickets = 0;
    if keylog {
        server_cfg.key_log = Arc::new(PrintKeyLog);
    }
    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).expect("bind");
    let server_cfg = Arc::new(server_cfg);
    std::thread::spawn(move || {
        let (mut tcp, _) = match listener.accept() {
            Ok(p) => p,
            Err(_) => return,
        };
        let _ = tcp.set_read_timeout(Some(Duration::from_secs(1800)));
        let mut conn = rustls::ServerConnection::new(server_cfg).unwrap();
        loop {
            while conn.wants_write() {
                if conn.write_tls(&mut tcp).is_err() {
                    return;
                }
            }
            if !conn.is_handshaking() {
                break;
            }
            match conn.read_tls(&mut tcp) {
                Ok(0) => return,
                Ok(_) => {
                    if let Err(e) = conn.process_new_packets() {
                        eprintln!("rustls TLS 1.3 process_new_packets: {e}");
                        return;
                    }
                }
                Err(_) => return,
            }
        }
        while conn.wants_write() {
            let _ = conn.write_tls(&mut tcp);
        }
        eprintln!("rustls TLS 1.3 handshake complete");
        if !serve_http {
            return;
        }
        loop {
            while conn.wants_write() {
                if conn.write_tls(&mut tcp).is_err() {
                    return;
                }
            }
            let mut buf = [0u8; 4096];
            match conn.reader().read(&mut buf) {
                Ok(0) => return,
                Ok(n) => {
                    if buf[..n].starts_with(b"GET ")
                        || buf[..n].starts_with(b"POST ")
                        || buf[..n].starts_with(b"PUT ")
                        || buf[..n].starts_with(b"PATCH ")
                        || buf[..n].starts_with(b"DELETE ")
                        || buf[..n].starts_with(b"HEAD ")
                        || buf[..n].starts_with(b"OPTIONS ")
                        || buf[..n].starts_with(b"TRACE ")
                        || buf[..n].starts_with(b"CONNECT ")
                    {
                        let _ = conn.writer().write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK",
                        );
                        let _ = conn.writer().flush();
                        while conn.wants_write() {
                            let _ = conn.write_tls(&mut tcp);
                        }
                    }
                    return;
                }
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    match conn.read_tls(&mut tcp) {
                        Ok(0) => return,
                        Ok(_) => {
                            if let Err(e) = conn.process_new_packets() {
                                eprintln!("rustls TLS 1.3 process_new_packets: {e}");
                                return;
                            }
                        }
                        Err(_) => return,
                    }
                }
                Err(_) => return,
            }
        }
    });
}

#[test]
fn sh23_rust_tls12_finished_against_rustls() {
    spawn_tls12_peer(28231, false, false);
    std::thread::sleep(Duration::from_millis(50));

    let mut tcp = TcpStream::connect("127.0.0.1:28231").unwrap();
    tcp.set_read_timeout(Some(Duration::from_secs(10))).ok();

    let client_random = [b'C'; 32];
    let kx = rustls::crypto::ring::kx_group::X25519
        .start()
        .expect("kx start");
    let client_pub = kx.pub_key().to_vec();

    let mut ch_body = Vec::new();
    ch_body.extend_from_slice(&[0x03, 0x03]);
    ch_body.extend_from_slice(&client_random);
    ch_body.extend_from_slice(&[
        0x00, 0x00, 0x04, 0x00, 0x9c, 0xc0, 0x2b, 0x01, 0x00, 0x00, 0x1d, 0x00, 0x0a, 0x00, 0x04,
        0x00, 0x02, 0x00, 0x1d, 0x00, 0x0b, 0x00, 0x02, 0x01, 0x00, 0x00, 0x0d, 0x00, 0x06, 0x00,
        0x04, 0x04, 0x01, 0x04, 0x03, 0xff, 0x01, 0x00, 0x01, 0x00,
    ]);
    let mut ch = vec![1, 0, 0, 0];
    let n = ch_body.len();
    ch[1] = ((n >> 16) & 255) as u8;
    ch[2] = ((n >> 8) & 255) as u8;
    ch[3] = (n & 255) as u8;
    ch.extend_from_slice(&ch_body);
    let mut rec = vec![22, 3, 3, (ch.len() >> 8) as u8, (ch.len() & 255) as u8];
    rec.extend_from_slice(&ch);
    tcp.write_all(&rec).unwrap();

    let mut hs_buf = Vec::new();
    let mut sh = None;
    let mut cert = None;
    let mut ske = None;
    let mut done = None;
    while done.is_none() {
        let (typ, payload) = read_record(&mut tcp);
        assert_eq!(typ, 22);
        hs_buf.extend_from_slice(&payload);
        while let Some(msg) = take_hs(&mut hs_buf) {
            match msg[0] {
                2 => sh = Some(msg),
                11 => cert = Some(msg),
                12 => ske = Some(msg),
                14 => done = Some(msg),
                _ => {}
            }
        }
    }
    let sh = sh.expect("sh");
    let cert = cert.expect("cert");
    let ske = ske.expect("ske");
    let done = done.expect("done");
    let session_len = sh[4 + 34] as usize;
    let cipher_off = 4 + 35 + session_len;
    let cipher = u16::from_be_bytes([sh[cipher_off], sh[cipher_off + 1]]);
    assert_eq!(cipher, 0xc02b);
    let server_random = sh[6..38].to_vec();

    let ske_body = &ske[4..];
    assert_eq!(ske_body[0], 3);
    let group = u16::from_be_bytes([ske_body[1], ske_body[2]]);
    assert_eq!(group, 0x001d);
    let pub_len = ske_body[3] as usize;
    let server_pub = &ske_body[4..4 + pub_len];

    let pms = kx
        .complete(server_pub)
        .expect("complete")
        .secret_bytes()
        .to_vec();

    let mut cke_body = vec![client_pub.len() as u8];
    cke_body.extend_from_slice(&client_pub);
    let mut cke = vec![16, 0, 0, 0];
    let n = cke_body.len();
    cke[1] = ((n >> 16) & 255) as u8;
    cke[2] = ((n >> 8) & 255) as u8;
    cke[3] = (n & 255) as u8;
    cke.extend_from_slice(&cke_body);

    let mut transcript = Vec::new();
    transcript.extend_from_slice(&ch);
    transcript.extend_from_slice(&sh);
    transcript.extend_from_slice(&cert);
    transcript.extend_from_slice(&ske);
    transcript.extend_from_slice(&done);
    transcript.extend_from_slice(&cke);

    let mut ms_seed = Vec::new();
    ms_seed.extend_from_slice(&client_random);
    ms_seed.extend_from_slice(&server_random);
    let ms = tls_prf(&pms, b"master secret", &ms_seed, 48);
    let mut ke_seed = Vec::new();
    ke_seed.extend_from_slice(&server_random);
    ke_seed.extend_from_slice(&client_random);
    let kb = tls_prf(&ms, b"key expansion", &ke_seed, 40);
    let client_key = &kb[0..16];
    let client_iv = &kb[32..36];

    let hash = Sha256::digest(&transcript);
    let verify = tls_prf(&ms, b"client finished", &hash, 12);
    let mut finished = vec![20, 0, 0, 12];
    finished.extend_from_slice(&verify);

    let mut aad = [0u8; 13];
    aad[8] = 22;
    aad[9] = 3;
    aad[10] = 3;
    aad[11] = 0;
    aad[12] = 16;
    let mut nonce = [0u8; 12];
    nonce[..4].copy_from_slice(client_iv);
    nonce[4..].copy_from_slice(&[0, 0, 0, 0, 0, 0, 0, 1]);
    let gcm = Aes128Gcm::new(client_key.into());
    let enc = gcm
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &finished,
                aad: &aad,
            },
        )
        .expect("gcm");
    let mut fin_rec = vec![22, 3, 3, 0, 40];
    fin_rec.extend_from_slice(&nonce[4..]);
    fin_rec.extend_from_slice(&enc);

    let mut cke_rec = vec![22, 3, 3, (cke.len() >> 8) as u8, (cke.len() & 255) as u8];
    cke_rec.extend_from_slice(&cke);
    tcp.write_all(&cke_rec).unwrap();
    tcp.write_all(&[0x14, 0x03, 0x03, 0x00, 0x01, 0x01]).unwrap();
    tcp.write_all(&fin_rec).unwrap();

    let mut saw_ccs = false;
    for _ in 0..8 {
        let (typ, payload) = read_record(&mut tcp);
        if typ == 20 {
            saw_ccs = true;
            continue;
        }
        if typ == 22 && saw_ccs {
            assert!(payload.len() >= 24, "encrypted finished");
            return;
        }
        if typ == 21 {
            panic!("tls alert {payload:?}");
        }
    }
    panic!("no server finished");
}

#[test]
fn sh23_rfc7748_x25519_vector() {
    let bob_pub = [
        0xde, 0x9e, 0xdb, 0x7d, 0x7b, 0x7d, 0xc1, 0xb4, 0xd3, 0x5b, 0x61, 0xc2, 0xec, 0xe4, 0x35,
        0x37, 0x3f, 0x83, 0x43, 0xc8, 0x5b, 0x78, 0x67, 0x4d, 0xad, 0xfc, 0x7e, 0x14, 0x6f, 0x88,
        0x2b, 0x4f,
    ];
    let k = x25519_rfc7748(ALICE_SECRET, bob_pub);
    let expect = [
        0x4a, 0x5d, 0x9d, 0x5b, 0xa4, 0xce, 0x2d, 0xe1, 0x72, 0x8e, 0x3b, 0xf4, 0x80, 0x35, 0x0f,
        0x25, 0xe0, 0x7e, 0x21, 0xc9, 0x47, 0xd1, 0x9e, 0x33, 0x76, 0xf0, 0x9b, 0x3c, 0x1e, 0x16,
        0x17, 0x42,
    ];
    assert_eq!(k, expect);
}

#[test]
fn sh23_rfc7748_alice_finished_against_rustls() {
    spawn_tls12_peer(28235, true, false);
    std::thread::sleep(Duration::from_millis(50));

    let mut tcp = TcpStream::connect("127.0.0.1:28235").unwrap();
    tcp.set_read_timeout(Some(Duration::from_secs(10))).ok();

    let client_random = [b'C'; 32];
    let client_pub = ALICE_PUB.to_vec();

    let mut ch_body = Vec::new();
    ch_body.extend_from_slice(&[0x03, 0x03]);
    ch_body.extend_from_slice(&client_random);
    ch_body.extend_from_slice(&[
        0x00, 0x00, 0x04, 0x00, 0x9c, 0xc0, 0x2b, 0x01, 0x00, 0x00, 0x1d, 0x00, 0x0a, 0x00, 0x04,
        0x00, 0x02, 0x00, 0x1d, 0x00, 0x0b, 0x00, 0x02, 0x01, 0x00, 0x00, 0x0d, 0x00, 0x06, 0x00,
        0x04, 0x04, 0x01, 0x04, 0x03, 0xff, 0x01, 0x00, 0x01, 0x00,
    ]);
    let mut ch = vec![1, 0, 0, 0];
    let n = ch_body.len();
    ch[1] = ((n >> 16) & 255) as u8;
    ch[2] = ((n >> 8) & 255) as u8;
    ch[3] = (n & 255) as u8;
    ch.extend_from_slice(&ch_body);
    let mut rec = vec![22, 3, 3, (ch.len() >> 8) as u8, (ch.len() & 255) as u8];
    rec.extend_from_slice(&ch);
    tcp.write_all(&rec).unwrap();

    let mut hs_buf = Vec::new();
    let mut sh = None;
    let mut cert = None;
    let mut ske = None;
    let mut done = None;
    while done.is_none() {
        let (typ, payload) = read_record(&mut tcp);
        assert_eq!(typ, 22);
        hs_buf.extend_from_slice(&payload);
        while let Some(msg) = take_hs(&mut hs_buf) {
            match msg[0] {
                2 => sh = Some(msg),
                11 => cert = Some(msg),
                12 => ske = Some(msg),
                14 => done = Some(msg),
                _ => {}
            }
        }
    }
    let sh = sh.expect("sh");
    let cert = cert.expect("cert");
    let ske = ske.expect("ske");
    let done = done.expect("done");
    let session_len = sh[4 + 34] as usize;
    let cipher_off = 4 + 35 + session_len;
    let cipher = u16::from_be_bytes([sh[cipher_off], sh[cipher_off + 1]]);
    assert_eq!(cipher, 0xc02b);
    let server_random = sh[6..38].to_vec();

    let ske_body = &ske[4..];
    let pub_len = ske_body[3] as usize;
    let mut server_pub = [0u8; 32];
    server_pub.copy_from_slice(&ske_body[4..4 + pub_len]);
    let pms = x25519_rfc7748(ALICE_SECRET, server_pub);

    let mut cke_body = vec![client_pub.len() as u8];
    cke_body.extend_from_slice(&client_pub);
    let mut cke = vec![16, 0, 0, 0];
    let n = cke_body.len();
    cke[1] = ((n >> 16) & 255) as u8;
    cke[2] = ((n >> 8) & 255) as u8;
    cke[3] = (n & 255) as u8;
    cke.extend_from_slice(&cke_body);

    let mut transcript = Vec::new();
    transcript.extend_from_slice(&ch);
    transcript.extend_from_slice(&sh);
    transcript.extend_from_slice(&cert);
    transcript.extend_from_slice(&ske);
    transcript.extend_from_slice(&done);
    transcript.extend_from_slice(&cke);

    let mut ms_seed = Vec::new();
    ms_seed.extend_from_slice(&client_random);
    ms_seed.extend_from_slice(&server_random);
    let ms = tls_prf(&pms, b"master secret", &ms_seed, 48);
    eprint!("alice-pms ");
    for b in &pms {
        eprint!("{b:02x}");
    }
    eprintln!();
    eprint!("alice-ms ");
    for b in &ms {
        eprint!("{b:02x}");
    }
    eprintln!();
    let mut ke_seed = Vec::new();
    ke_seed.extend_from_slice(&server_random);
    ke_seed.extend_from_slice(&client_random);
    let kb = tls_prf(&ms, b"key expansion", &ke_seed, 40);
    let client_key = &kb[0..16];
    let client_iv = &kb[32..36];

    let hash = Sha256::digest(&transcript);
    let verify = tls_prf(&ms, b"client finished", &hash, 12);
    let mut finished = vec![20, 0, 0, 12];
    finished.extend_from_slice(&verify);

    let mut aad = [0u8; 13];
    aad[8] = 22;
    aad[9] = 3;
    aad[10] = 3;
    aad[11] = 0;
    aad[12] = 16;
    let mut nonce = [0u8; 12];
    nonce[..4].copy_from_slice(client_iv);
    nonce[4..].copy_from_slice(&[0, 0, 0, 0, 0, 0, 0, 1]);
    let gcm = Aes128Gcm::new(client_key.into());
    let enc = gcm
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &finished,
                aad: &aad,
            },
        )
        .expect("gcm");
    let mut fin_rec = vec![22, 3, 3, 0, 40];
    fin_rec.extend_from_slice(&nonce[4..]);
    fin_rec.extend_from_slice(&enc);

    let mut cke_rec = vec![22, 3, 3, (cke.len() >> 8) as u8, (cke.len() & 255) as u8];
    cke_rec.extend_from_slice(&cke);
    tcp.write_all(&cke_rec).unwrap();
    tcp.write_all(&[0x14, 0x03, 0x03, 0x00, 0x01, 0x01]).unwrap();
    tcp.write_all(&fin_rec).unwrap();

    let mut saw_ccs = false;
    for _ in 0..8 {
        let (typ, payload) = read_record(&mut tcp);
        if typ == 20 {
            saw_ccs = true;
            continue;
        }
        if typ == 22 && saw_ccs {
            assert!(payload.len() >= 24, "encrypted finished");
            return;
        }
        if typ == 21 {
            panic!("tls alert {payload:?}");
        }
    }
    panic!("no server finished");
}

#[test]
fn sh23_crypto_tls_peer_fin_eval_smoke() {
    spawn_tls12_peer(28233, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls_peer_fin_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls-peer-fin".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile tls peer fin smoke");
            let value = eval_program(&program, &mut env).expect("run tls peer fin smoke");
            eprintln!("sh23 tls peer fin value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn sh23_crypto_tls_prf_keyseed_eval_smoke() {
    let secret: &[u8] = &[
        0x9b, 0xbe, 0x43, 0x6b, 0xa9, 0x40, 0xf0, 0x17, 0xb1, 0x76, 0x52, 0x84, 0x9a, 0x71, 0xdb,
        0x35,
    ];
    let seed = [
        0xa0, 0xba, 0x9f, 0x93, 0x6c, 0xda, 0x31, 0x18, 0x27, 0xa6, 0xf7, 0x96, 0xff, 0xd5, 0x19,
        0x8c,
    ];
    let expect = hex_of(&tls_prf(secret, b"test label", &seed, 48));
    let path = format!(
        "{}/examples/sh23_crypto_tls_prf_keyseed_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls-prf-keyseed".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile prf keyseed");
            let value = eval_program(&program, &mut env).expect("run prf keyseed");
            let got = match value {
                kabootar_lib::value::Value::String(s) => s,
                other => panic!("expected string {other:?}"),
            };
            eprintln!("kab prf {got}");
            eprintln!("rust prf {expect}");
            assert_eq!(got, expect);
        })
        .expect("spawn")
        .join()
        .expect("join");
}

#[test]
#[ignore]
fn sh23_crypto_x25519_alice_base_eval_smoke() {
    let path = format!(
        "{}/examples/sh23_crypto_x25519_alice_base_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-x25519-alice-base".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile alice base");
            let value = eval_program(&program, &mut env).expect("run alice base");
            let got = match value {
                kabootar_lib::value::Value::String(s) => s,
                other => panic!("expected string {other:?}"),
            };
            eprintln!("kab alice*9 {got}");
            eprintln!("rfc alice   8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a");
            assert_eq!(
                got,
                "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a"
            );
        })
        .expect("spawn")
        .join()
        .expect("join");
}

#[test]
fn sh23_crypto_tls_peer_get_eval_smoke() {
    spawn_tls12_peer(28261, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls_peer_get_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls-peer-get".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile tls peer get");
            let value = eval_program(&program, &mut env).expect("run tls peer get");
            eprintln!("sh23 tls peer get value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

#[test]
fn sh23_crypto_http_fetch_peer_eval_smoke() {
    spawn_tls12_peer(28291, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_peer_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-peer".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile httpFetch peer");
            let value = eval_program(&program, &mut env).expect("run httpFetch peer");
            eprintln!("sh23 httpFetch peer value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: product httpFetch routes Kab TLS loop (:28199) and rustls peer (:28291).
#[test]
fn sh23_crypto_http_fetch_route_eval_smoke() {
    spawn_tls12_peer(28291, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_route_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-route".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile httpFetch route");
            let value = eval_program(&program, &mut env).expect("run httpFetch route");
            eprintln!("sh23 httpFetch route value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: product wrapper httpFetch after crypto bind (cheap smokes do not import crypto).
#[test]
fn sh23_crypto_http_fetch_bind_eval_smoke() {
    spawn_tls12_peer(28291, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_bind_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-bind".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile httpFetch bind");
            let value = eval_program(&program, &mut env).expect("run httpFetch bind");
            eprintln!("sh23 httpFetch bind value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: await wrapper httpFetch over Kab TLS loop and rustls peer (not a hanging sync dict).
#[test]
fn sh23_crypto_http_fetch_await_eval_smoke() {
    spawn_tls12_peer(28291, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_await_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-await".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile await httpFetch");
            let value = eval_program(&program, &mut env).expect("run await httpFetch");
            eprintln!("sh23 await httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: http_status/http_body on wrapper httpFetch over Kab TLS (same shape as rustls).
#[test]
fn sh23_crypto_http_fetch_body_eval_smoke() {
    spawn_tls12_peer(28291, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_body_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-body".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile http_body httpFetch");
            let value = eval_program(&program, &mut env).expect("run http_body httpFetch");
            eprintln!("sh23 http_body httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: http_header/http_headers on wrapper httpFetch over Kab TLS (wire headers, not rustls-delete).
#[test]
fn sh23_crypto_http_fetch_hdr_eval_smoke() {
    spawn_tls12_peer(28291, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_hdr_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-hdr".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile http_header httpFetch");
            let value = eval_program(&program, &mut env).expect("run http_header httpFetch");
            eprintln!("sh23 http_header httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch over Kab TLS for a path other than /v?q=1 (not rustls-host).
#[test]
fn sh23_crypto_http_fetch_url_eval_smoke() {
    spawn_tls12_peer(28291, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_url_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-url".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile url httpFetch");
            let value = eval_program(&program, &mut env).expect("run url httpFetch");
            eprintln!("sh23 url httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch over Kab TLS for localhost (not just 127.0.0.1; not rustls-host).
#[test]
fn sh23_crypto_http_fetch_localhost_eval_smoke() {
    spawn_tls12_peer(28291, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_localhost_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-localhost".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile localhost httpFetch");
            let value = eval_program(&program, &mut env).expect("run localhost httpFetch");
            eprintln!("sh23 localhost httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper POST httpFetch over Kab TLS loop and rustls peer (not rustls-host fetch).
#[test]
fn sh23_crypto_http_fetch_post_eval_smoke() {
    spawn_tls12_peer(28291, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_post_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-post".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile POST httpFetch");
            let value = eval_program(&program, &mut env).expect("run POST httpFetch");
            eprintln!("sh23 POST httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper PUT httpFetch over Kab TLS loop and rustls peer (not rustls-host fetch).
#[test]
fn sh23_crypto_http_fetch_put_eval_smoke() {
    spawn_tls12_peer(28291, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_put_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-put".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile PUT httpFetch");
            let value = eval_program(&program, &mut env).expect("run PUT httpFetch");
            eprintln!("sh23 PUT httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper PATCH httpFetch over Kab TLS loop and rustls peer (not rustls-host fetch).
#[test]
fn sh23_crypto_http_fetch_patch_eval_smoke() {
    spawn_tls12_peer(28291, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_patch_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-patch".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile PATCH httpFetch");
            let value = eval_program(&program, &mut env).expect("run PATCH httpFetch");
            eprintln!("sh23 PATCH httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper DELETE httpFetch over Kab TLS loop and rustls peer (not rustls-host fetch).
#[test]
fn sh23_crypto_http_fetch_delete_eval_smoke() {
    spawn_tls12_peer(28291, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_delete_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-delete".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile DELETE httpFetch");
            let value = eval_program(&program, &mut env).expect("run DELETE httpFetch");
            eprintln!("sh23 DELETE httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper HEAD httpFetch over Kab TLS loop and rustls peer (not rustls-host fetch).
#[test]
fn sh23_crypto_http_fetch_head_eval_smoke() {
    spawn_tls12_peer(28291, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_head_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-head".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile HEAD httpFetch");
            let value = eval_program(&program, &mut env).expect("run HEAD httpFetch");
            eprintln!("sh23 HEAD httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper OPTIONS httpFetch over Kab TLS loop and rustls peer (not rustls-host fetch).
#[test]
fn sh23_crypto_http_fetch_options_eval_smoke() {
    spawn_tls12_peer(28291, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_options_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-options".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile OPTIONS httpFetch");
            let value = eval_program(&program, &mut env).expect("run OPTIONS httpFetch");
            eprintln!("sh23 OPTIONS httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper TRACE httpFetch over Kab TLS loop and rustls peer (not rustls-host fetch).
#[test]
fn sh23_crypto_http_fetch_trace_eval_smoke() {
    spawn_tls12_peer(28291, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_trace_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-trace".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TRACE httpFetch");
            let value = eval_program(&program, &mut env).expect("run TRACE httpFetch");
            eprintln!("sh23 TRACE httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper CONNECT httpFetch over Kab TLS loop and rustls peer (not rustls-host fetch).
#[test]
fn sh23_crypto_http_fetch_connect_eval_smoke() {
    spawn_tls12_peer(28291, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_connect_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-connect".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile CONNECT httpFetch");
            let value = eval_program(&program, &mut env).expect("run CONNECT httpFetch");
            eprintln!("sh23 CONNECT httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper GET httpFetch with Authorization over Kab TLS loop and rustls peer.
#[test]
fn sh23_crypto_http_fetch_auth_eval_smoke() {
    spawn_tls12_peer(28291, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_auth_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-auth".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile Authorization httpFetch");
            let value = eval_program(&program, &mut env).expect("run Authorization httpFetch");
            eprintln!("sh23 Authorization httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper GET httpFetch with Cookie over Kab TLS loop and rustls peer.
#[test]
fn sh23_crypto_http_fetch_cookie_eval_smoke() {
    spawn_tls12_peer(28291, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_cookie_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-cookie".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile Cookie httpFetch");
            let value = eval_program(&program, &mut env).expect("run Cookie httpFetch");
            eprintln!("sh23 Cookie httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper GET httpFetch with Proxy-Authorization over Kab TLS loop and rustls peer.
#[test]
fn sh23_crypto_http_fetch_proxy_eval_smoke() {
    spawn_tls12_peer(28291, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_proxy_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-proxy".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile Proxy-Authorization httpFetch");
            let value = eval_program(&program, &mut env).expect("run Proxy-Authorization httpFetch");
            eprintln!("sh23 Proxy-Authorization httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: live TLS 1.3 ClientHello/ServerHello on loopback (0x1301, 0x0304, X25519).
#[test]
fn sh23_crypto_tls13_hs_eval_smoke() {
    let path = format!(
        "{}/examples/sh23_crypto_tls13_hs_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-hs".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 hs eval smoke");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 hs eval smoke");
            eprintln!("sh23 TLS 1.3 hs value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: TLS 1.3 HKDF + EncryptedExtensions/Finished on loopback (not rustls-host).
#[test]
fn sh23_crypto_tls13_fin_eval_smoke() {
    let path = format!(
        "{}/examples/sh23_crypto_tls13_fin_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-fin".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 fin eval smoke");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 fin eval smoke");
            eprintln!("sh23 TLS 1.3 fin value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: TLS 1.3 AES-GCM handshake records (0x17) for EE/Finished.
#[test]
fn sh23_crypto_tls13_gcm_eval_smoke() {
    let path = format!(
        "{}/examples/sh23_crypto_tls13_gcm_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-gcm".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 gcm eval smoke");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 gcm eval smoke");
            eprintln!("sh23 TLS 1.3 gcm value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: TLS 1.3 application-data GET after GCM Finished (loopback, not rustls-host).
#[test]
fn sh23_crypto_tls13_app_eval_smoke() {
    let path = format!(
        "{}/examples/sh23_crypto_tls13_app_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-app".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 app eval smoke");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 app eval smoke");
            eprintln!("sh23 TLS 1.3 app value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch GET over Kab TLS 1.3 loopback :28198 (not rustls-host).
#[test]
fn sh23_crypto_http_fetch_tls13_eval_smoke() {
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 httpFetch");
            eprintln!("sh23 TLS 1.3 httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch GET over Kab TLS 1.3 localhost :28198 (not rustls-host).
#[test]
fn sh23_crypto_http_fetch_tls13_localhost_eval_smoke() {
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_localhost_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-localhost".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 localhost httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 localhost httpFetch");
            eprintln!("sh23 TLS 1.3 localhost httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch GET other path over Kab TLS 1.3 :28198 (not rustls-host).
#[test]
fn sh23_crypto_http_fetch_tls13_url_eval_smoke() {
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_url_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-url".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 path httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 path httpFetch");
            eprintln!("sh23 TLS 1.3 path httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch POST over Kab TLS 1.3 :28198 (not rustls-host).
#[test]
fn sh23_crypto_http_fetch_tls13_post_eval_smoke() {
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_post_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-post".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 POST httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 POST httpFetch");
            eprintln!("sh23 TLS 1.3 POST httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch PUT over Kab TLS 1.3 :28198 (not rustls-host).
#[test]
fn sh23_crypto_http_fetch_tls13_put_eval_smoke() {
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_put_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-put".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 PUT httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 PUT httpFetch");
            eprintln!("sh23 TLS 1.3 PUT httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch PATCH over Kab TLS 1.3 :28198 (not rustls-host).
#[test]
fn sh23_crypto_http_fetch_tls13_patch_eval_smoke() {
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_patch_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-patch".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 PATCH httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 PATCH httpFetch");
            eprintln!("sh23 TLS 1.3 PATCH httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch DELETE over Kab TLS 1.3 :28198 (not rustls-host).
#[test]
fn sh23_crypto_http_fetch_tls13_delete_eval_smoke() {
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_delete_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-delete".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 DELETE httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 DELETE httpFetch");
            eprintln!("sh23 TLS 1.3 DELETE httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch HEAD over Kab TLS 1.3 :28198 (not rustls-host).
#[test]
fn sh23_crypto_http_fetch_tls13_head_eval_smoke() {
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_head_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-head".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 HEAD httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 HEAD httpFetch");
            eprintln!("sh23 TLS 1.3 HEAD httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch OPTIONS over Kab TLS 1.3 :28198 (not rustls-host).
#[test]
fn sh23_crypto_http_fetch_tls13_options_eval_smoke() {
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_options_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-options".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 OPTIONS httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 OPTIONS httpFetch");
            eprintln!("sh23 TLS 1.3 OPTIONS httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch TRACE over Kab TLS 1.3 :28198 (not rustls-host).
#[test]
fn sh23_crypto_http_fetch_tls13_trace_eval_smoke() {
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_trace_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-trace".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 TRACE httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 TRACE httpFetch");
            eprintln!("sh23 TLS 1.3 TRACE httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch CONNECT over Kab TLS 1.3 :28198 (authority-form, not rustls-host).
#[test]
fn sh23_crypto_http_fetch_tls13_connect_eval_smoke() {
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_connect_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-connect".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 CONNECT httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 CONNECT httpFetch");
            eprintln!("sh23 TLS 1.3 CONNECT httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch GET Authorization over Kab TLS 1.3 :28198 (not rustls-host).
#[test]
fn sh23_crypto_http_fetch_tls13_auth_eval_smoke() {
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_auth_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-auth".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 Authorization httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 Authorization httpFetch");
            eprintln!("sh23 TLS 1.3 Authorization httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch GET Cookie over Kab TLS 1.3 :28198 (not rustls-host).
#[test]
fn sh23_crypto_http_fetch_tls13_cookie_eval_smoke() {
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_cookie_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-cookie".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 Cookie httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 Cookie httpFetch");
            eprintln!("sh23 TLS 1.3 Cookie httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch GET Proxy-Authorization over Kab TLS 1.3 :28198 (not rustls-host).
#[test]
fn sh23_crypto_http_fetch_tls13_proxy_eval_smoke() {
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_proxy_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-proxy".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 Proxy-Authorization httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 Proxy-Authorization httpFetch");
            eprintln!("sh23 TLS 1.3 Proxy-Authorization httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 ClientHello/ServerHello vs rustls TLS 1.3 peer :28293 (not rustls-host fetch).
#[test]
fn sh23_crypto_tls13_peer_eval_smoke() {
    spawn_tls13_peer(28293, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer hello");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer hello");
            eprintln!("sh23 TLS 1.3 rustls peer hello value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 EncryptedExtensions/Finished vs rustls TLS 1.3 peer :28294.
#[test]
fn sh23_crypto_tls13_peer_fin_eval_smoke() {
    spawn_tls13_peer(28294, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_fin_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-fin".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer Finished");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer Finished");
            eprintln!("sh23 TLS 1.3 rustls peer Finished value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 application-data GET vs rustls TLS 1.3 peer :28295.
#[test]
fn sh23_crypto_tls13_peer_get_eval_smoke() {
    spawn_tls13_peer(28295, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_get_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-get".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer GET");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer GET");
            eprintln!("sh23 TLS 1.3 rustls peer GET value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch GET over Kab TLS 1.3 vs rustls peer :28296 (not rustls-host fetch).
#[test]
fn sh23_crypto_http_fetch_tls13_peer_eval_smoke() {
    spawn_tls13_peer(28296, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_peer_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-peer".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer httpFetch");
            eprintln!("sh23 TLS 1.3 rustls peer httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch GET over Kab TLS 1.3 vs rustls peer localhost :28296.
#[test]
fn sh23_crypto_http_fetch_tls13_peer_localhost_eval_smoke() {
    spawn_tls13_peer(28296, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_peer_localhost_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-peer-localhost".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer localhost httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer localhost httpFetch");
            eprintln!("sh23 TLS 1.3 rustls peer localhost httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch GET other path over Kab TLS 1.3 vs rustls peer :28296.
#[test]
fn sh23_crypto_http_fetch_tls13_peer_url_eval_smoke() {
    spawn_tls13_peer(28296, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_peer_url_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-peer-url".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer URL httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer URL httpFetch");
            eprintln!("sh23 TLS 1.3 rustls peer URL httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch POST over Kab TLS 1.3 vs rustls peer :28296.
#[test]
fn sh23_crypto_http_fetch_tls13_peer_post_eval_smoke() {
    spawn_tls13_peer(28296, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_peer_post_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-peer-post".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer POST httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer POST httpFetch");
            eprintln!("sh23 TLS 1.3 rustls peer POST httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch PUT over Kab TLS 1.3 vs rustls peer :28296.
#[test]
fn sh23_crypto_http_fetch_tls13_peer_put_eval_smoke() {
    spawn_tls13_peer(28296, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_peer_put_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-peer-put".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer PUT httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer PUT httpFetch");
            eprintln!("sh23 TLS 1.3 rustls peer PUT httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch PATCH over Kab TLS 1.3 vs rustls peer :28296.
#[test]
fn sh23_crypto_http_fetch_tls13_peer_patch_eval_smoke() {
    spawn_tls13_peer(28296, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_peer_patch_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-peer-patch".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer PATCH httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer PATCH httpFetch");
            eprintln!("sh23 TLS 1.3 rustls peer PATCH httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch DELETE over Kab TLS 1.3 vs rustls peer :28296.
#[test]
fn sh23_crypto_http_fetch_tls13_peer_delete_eval_smoke() {
    spawn_tls13_peer(28296, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_peer_delete_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-peer-delete".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer DELETE httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer DELETE httpFetch");
            eprintln!("sh23 TLS 1.3 rustls peer DELETE httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch HEAD over Kab TLS 1.3 vs rustls peer :28296.
#[test]
fn sh23_crypto_http_fetch_tls13_peer_head_eval_smoke() {
    spawn_tls13_peer(28296, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_peer_head_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-peer-head".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer HEAD httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer HEAD httpFetch");
            eprintln!("sh23 TLS 1.3 rustls peer HEAD httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch OPTIONS over Kab TLS 1.3 vs rustls peer :28296.
#[test]
fn sh23_crypto_http_fetch_tls13_peer_options_eval_smoke() {
    spawn_tls13_peer(28296, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_peer_options_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-peer-options".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer OPTIONS httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer OPTIONS httpFetch");
            eprintln!("sh23 TLS 1.3 rustls peer OPTIONS httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch TRACE over Kab TLS 1.3 vs rustls peer :28296.
#[test]
fn sh23_crypto_http_fetch_tls13_peer_trace_eval_smoke() {
    spawn_tls13_peer(28296, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_peer_trace_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-peer-trace".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer TRACE httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer TRACE httpFetch");
            eprintln!("sh23 TLS 1.3 rustls peer TRACE httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch CONNECT over Kab TLS 1.3 vs rustls peer :28296 (authority-form).
#[test]
fn sh23_crypto_http_fetch_tls13_peer_connect_eval_smoke() {
    spawn_tls13_peer(28296, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_peer_connect_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-peer-connect".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer CONNECT httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer CONNECT httpFetch");
            eprintln!("sh23 TLS 1.3 rustls peer CONNECT httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch GET Authorization over Kab TLS 1.3 vs rustls peer :28296.
#[test]
fn sh23_crypto_http_fetch_tls13_peer_auth_eval_smoke() {
    spawn_tls13_peer(28296, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_peer_auth_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-peer-auth".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer Authorization httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer Authorization httpFetch");
            eprintln!("sh23 TLS 1.3 rustls peer Authorization httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch GET Cookie over Kab TLS 1.3 vs rustls peer :28296.
#[test]
fn sh23_crypto_http_fetch_tls13_peer_cookie_eval_smoke() {
    spawn_tls13_peer(28296, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_peer_cookie_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-peer-cookie".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer Cookie httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer Cookie httpFetch");
            eprintln!("sh23 TLS 1.3 rustls peer Cookie httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: wrapper httpFetch GET Proxy-Authorization over Kab TLS 1.3 vs rustls peer :28296.
#[test]
fn sh23_crypto_http_fetch_tls13_peer_proxy_eval_smoke() {
    spawn_tls13_peer(28296, true, true);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_http_fetch_tls13_peer_proxy_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-http-fetch-tls13-peer-proxy".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer Proxy-Authorization httpFetch");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer Proxy-Authorization httpFetch");
            eprintln!("sh23 TLS 1.3 rustls peer Proxy-Authorization httpFetch value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 Certificate/CertificateVerify vs rustls TLS 1.3 peer :28297.
#[test]
fn sh23_crypto_tls13_peer_cv_eval_smoke() {
    spawn_tls13_peer(28297, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_cv_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-cv".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer CertificateVerify");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer CertificateVerify");
            eprintln!("sh23 TLS 1.3 rustls peer CertificateVerify value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 ECDSA-SHA256 verify of CertificateVerify vs rustls peer :28299.
#[test]
fn sh23_crypto_tls13_peer_ecdsa_eval_smoke() {
    spawn_tls13_peer(28299, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_ecdsa_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-ecdsa".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer ECDSA CertificateVerify");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer ECDSA CertificateVerify");
            eprintln!("sh23 TLS 1.3 rustls peer ECDSA CertificateVerify value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 RSA-PSS SHA-256 verify of CertificateVerify vs rustls RSA peer :28300.
#[test]
fn sh23_crypto_tls13_peer_pss_eval_smoke() {
    spawn_tls13_rsa_peer(28300);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_pss_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-pss".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer RSA-PSS CertificateVerify");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer RSA-PSS CertificateVerify");
            eprintln!("sh23 TLS 1.3 rustls peer RSA-PSS CertificateVerify value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 PKCS#1 v1.5 SHA-256 verify of leaf TBS vs rustls RSA peer :28301.
#[test]
fn sh23_crypto_tls13_peer_tbs_eval_smoke() {
    spawn_tls13_rsa_peer(28301);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_tbs_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-tbs".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer RSA leaf TBS");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer RSA leaf TBS");
            eprintln!("sh23 TLS 1.3 rustls peer RSA leaf TBS value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 ECDSA-SHA256 verify of leaf TBS vs rustls P-256 peer :28302.
#[test]
fn sh23_crypto_tls13_peer_ecdsa_tbs_eval_smoke() {
    spawn_tls13_peer(28302, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_ecdsa_tbs_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-ecdsa-tbs".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer ECDSA leaf TBS");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer ECDSA leaf TBS");
            eprintln!("sh23 TLS 1.3 rustls peer ECDSA leaf TBS value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf SAN iPAddress 127.0.0.1 vs rustls P-256 peer :28303.
#[test]
fn sh23_crypto_tls13_peer_san_eval_smoke() {
    spawn_tls13_peer(28303, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_san_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-san".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf SAN");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf SAN");
            eprintln!("sh23 TLS 1.3 rustls peer leaf SAN value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf Validity notBefore/notAfter vs rustls P-256 peer :28304.
#[test]
fn sh23_crypto_tls13_peer_time_eval_smoke() {
    spawn_tls13_peer(28304, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_time_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-time".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf validity");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf validity");
            eprintln!("sh23 TLS 1.3 rustls peer leaf validity value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf keyUsage digitalSignature vs rustls P-256 peer :28305.
#[test]
fn sh23_crypto_tls13_peer_ku_eval_smoke() {
    spawn_tls13_peer(28305, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_ku_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-ku".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf keyUsage");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf keyUsage");
            eprintln!("sh23 TLS 1.3 rustls peer leaf keyUsage value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf EKU serverAuth vs rustls P-256 peer :28306.
#[test]
fn sh23_crypto_tls13_peer_eku_eval_smoke() {
    spawn_tls13_peer(28306, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_eku_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-eku".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf EKU");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf EKU");
            eprintln!("sh23 TLS 1.3 rustls peer leaf EKU value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf basicConstraints cA=FALSE vs rustls P-256 peer :28307.
#[test]
fn sh23_crypto_tls13_peer_bc_eval_smoke() {
    spawn_tls13_peer(28307, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_bc_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-bc".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf basicConstraints");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf basicConstraints");
            eprintln!("sh23 TLS 1.3 rustls peer leaf basicConstraints value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf issuer=subject Name vs rustls P-256 peer :28308.
#[test]
fn sh23_crypto_tls13_peer_iss_eval_smoke() {
    spawn_tls13_peer(28308, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_iss_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-iss".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf issuer=subject");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf issuer=subject");
            eprintln!("sh23 TLS 1.3 rustls peer leaf issuer=subject value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf serial INTEGER vs rustls P-256 peer :28309.
#[test]
fn sh23_crypto_tls13_peer_sn_eval_smoke() {
    spawn_tls13_peer(28309, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_sn_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-sn".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf serial");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf serial");
            eprintln!("sh23 TLS 1.3 rustls peer leaf serial value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf version v3 vs rustls P-256 peer :28310.
#[test]
fn sh23_crypto_tls13_peer_ver_eval_smoke() {
    spawn_tls13_peer(28310, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_ver_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-ver".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf version v3");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf version v3");
            eprintln!("sh23 TLS 1.3 rustls peer leaf version v3 value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf subjectKeyIdentifier vs rustls P-256 peer :28311.
#[test]
fn sh23_crypto_tls13_peer_ski_eval_smoke() {
    spawn_tls13_peer(28311, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_ski_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-ski".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf SKI");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf SKI");
            eprintln!("sh23 TLS 1.3 rustls peer leaf SKI value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf authorityKeyIdentifier vs rustls P-256 peer :28312.
#[test]
fn sh23_crypto_tls13_peer_aki_eval_smoke() {
    spawn_tls13_peer(28312, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_aki_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-aki".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf AKI");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf AKI");
            eprintln!("sh23 TLS 1.3 rustls peer leaf AKI value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf SPKI id-ecPublicKey vs rustls P-256 peer :28313.
#[test]
fn sh23_crypto_tls13_peer_spki_eval_smoke() {
    spawn_tls13_peer(28313, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_spki_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-spki".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf SPKI");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf SPKI");
            eprintln!("sh23 TLS 1.3 rustls peer leaf SPKI value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf SPKI secp256r1 vs rustls P-256 peer :28314.
#[test]
fn sh23_crypto_tls13_peer_p256_eval_smoke() {
    spawn_tls13_peer(28314, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_p256_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-p256".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf secp256r1");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf secp256r1");
            eprintln!("sh23 TLS 1.3 rustls peer leaf secp256r1 value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf SPKI uncompressed P-256 point vs rustls peer :28315.
#[test]
fn sh23_crypto_tls13_peer_pt_eval_smoke() {
    spawn_tls13_peer(28315, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_pt_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-pt".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf uncompressed point");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf uncompressed point");
            eprintln!("sh23 TLS 1.3 rustls peer leaf uncompressed point value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf SPKI nonzero P-256 X/Y vs rustls peer :28316.
#[test]
fn sh23_crypto_tls13_peer_xy_eval_smoke() {
    spawn_tls13_peer(28316, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_xy_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-xy".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf nonzero X/Y");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf nonzero X/Y");
            eprintln!("sh23 TLS 1.3 rustls peer leaf nonzero X/Y value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf subject commonName vs rustls P-256 peer :28317.
#[test]
fn sh23_crypto_tls13_peer_cn_eval_smoke() {
    spawn_tls13_peer(28317, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_cn_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-cn".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf CN");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf CN");
            eprintln!("sh23 TLS 1.3 rustls peer leaf CN value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf CN UTF8String vs rustls P-256 peer :28318.
#[test]
fn sh23_crypto_tls13_peer_utf8_eval_smoke() {
    spawn_tls13_peer(28318, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_utf8_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-utf8".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf CN UTF8String");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf CN UTF8String");
            eprintln!("sh23 TLS 1.3 rustls peer leaf CN UTF8String value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf CN UTF8 bytes vs rustls P-256 peer :28319.
#[test]
fn sh23_crypto_tls13_peer_cnb_eval_smoke() {
    spawn_tls13_peer(28319, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_cnb_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-cnb".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf CN bytes");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf CN bytes");
            eprintln!("sh23 TLS 1.3 rustls peer leaf CN bytes value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf CN UTF8 no-NUL vs rustls P-256 peer :28320.
#[test]
fn sh23_crypto_tls13_peer_nul_eval_smoke() {
    spawn_tls13_peer(28320, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_nul_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-nul".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf CN no-NUL");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf CN no-NUL");
            eprintln!("sh23 TLS 1.3 rustls peer leaf CN no-NUL value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf CN printable ASCII vs rustls P-256 peer :28321.
#[test]
fn sh23_crypto_tls13_peer_prn_eval_smoke() {
    spawn_tls13_peer(28321, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_prn_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-prn".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf CN printable");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf CN printable");
            eprintln!("sh23 TLS 1.3 rustls peer leaf CN printable value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf CN trim vs rustls P-256 peer :28322.
#[test]
fn sh23_crypto_tls13_peer_trim_eval_smoke() {
    spawn_tls13_peer(28322, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_trim_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-trim".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf CN trim");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf CN trim");
            eprintln!("sh23 TLS 1.3 rustls peer leaf CN trim value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf CN no double-space vs rustls P-256 peer :28323.
#[test]
fn sh23_crypto_tls13_peer_spc_eval_smoke() {
    spawn_tls13_peer(28323, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_spc_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-spc".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf CN no double-space");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf CN no double-space");
            eprintln!("sh23 TLS 1.3 rustls peer leaf CN no double-space value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf CN length ≤64 vs rustls P-256 peer :28324.
#[test]
fn sh23_crypto_tls13_peer_cnl_eval_smoke() {
    spawn_tls13_peer(28324, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_cnl_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-cnl".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf CN length");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf CN length");
            eprintln!("sh23 TLS 1.3 rustls peer leaf CN length value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf CN has letter vs rustls P-256 peer :28325.
#[test]
fn sh23_crypto_tls13_peer_ltr_eval_smoke() {
    spawn_tls13_peer(28325, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_ltr_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-ltr".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf CN letter");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf CN letter");
            eprintln!("sh23 TLS 1.3 rustls peer leaf CN letter value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf CN has space vs rustls P-256 peer :28326.
#[test]
fn sh23_crypto_tls13_peer_wsp_eval_smoke() {
    spawn_tls13_peer(28326, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_wsp_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-wsp".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf CN space");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf CN space");
            eprintln!("sh23 TLS 1.3 rustls peer leaf CN space value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf CN no digit vs rustls P-256 peer :28327.
#[test]
fn sh23_crypto_tls13_peer_dig_eval_smoke() {
    spawn_tls13_peer(28327, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_dig_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-dig".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf CN no digit");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf CN no digit");
            eprintln!("sh23 TLS 1.3 rustls peer leaf CN no digit value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf CN no punctuation vs rustls P-256 peer :28328.
#[test]
fn sh23_crypto_tls13_peer_pun_eval_smoke() {
    spawn_tls13_peer(28328, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_pun_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-pun".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf CN no punctuation");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf CN no punctuation");
            eprintln!("sh23 TLS 1.3 rustls peer leaf CN no punctuation value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf CN no uppercase vs rustls P-256 peer :28329.
#[test]
fn sh23_crypto_tls13_peer_upc_eval_smoke() {
    spawn_tls13_peer(28329, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_upc_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-upc".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf CN no uppercase");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf CN no uppercase");
            eprintln!("sh23 TLS 1.3 rustls peer leaf CN no uppercase value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf CN four words vs rustls P-256 peer :28330.
#[test]
fn sh23_crypto_tls13_peer_wrd_eval_smoke() {
    spawn_tls13_peer(28330, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_wrd_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-wrd".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf CN four words");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf CN four words");
            eprintln!("sh23 TLS 1.3 rustls peer leaf CN four words value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf CN starts with r vs rustls P-256 peer :28331.
#[test]
fn sh23_crypto_tls13_peer_str_eval_smoke() {
    spawn_tls13_peer(28331, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_str_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-str".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf CN starts with r");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf CN starts with r");
            eprintln!("sh23 TLS 1.3 rustls peer leaf CN starts with r value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf CN ends with t vs rustls P-256 peer :28332.
#[test]
fn sh23_crypto_tls13_peer_end_eval_smoke() {
    spawn_tls13_peer(28332, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_end_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-end".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf CN ends with t");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf CN ends with t");
            eprintln!("sh23 TLS 1.3 rustls peer leaf CN ends with t value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf CN prefix rcgen vs rustls P-256 peer :28333.
#[test]
fn sh23_crypto_tls13_peer_pfx_eval_smoke() {
    spawn_tls13_peer(28333, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_pfx_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-pfx".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf CN prefix rcgen");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf CN prefix rcgen");
            eprintln!("sh23 TLS 1.3 rustls peer leaf CN prefix rcgen value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf CN suffix cert vs rustls P-256 peer :28350.
#[test]
fn sh23_crypto_tls13_peer_sfx_eval_smoke() {
    spawn_tls13_peer(28350, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_sfx_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-sfx".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf CN suffix cert");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf CN suffix cert");
            eprintln!("sh23 TLS 1.3 rustls peer leaf CN suffix cert value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf CN word self vs rustls P-256 peer :28380.
#[test]
fn sh23_crypto_tls13_peer_slf_eval_smoke() {
    spawn_tls13_peer(28380, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_slf_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-slf".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf CN word self");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf CN word self");
            eprintln!("sh23 TLS 1.3 rustls peer leaf CN word self value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf CN word signed vs rustls P-256 peer :28381.
#[test]
fn sh23_crypto_tls13_peer_sgd_eval_smoke() {
    spawn_tls13_peer(28381, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_sgd_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-sgd".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf CN word signed");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf CN word signed");
            eprintln!("sh23 TLS 1.3 rustls peer leaf CN word signed value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf CN word cert vs rustls P-256 peer :28382.
#[test]
fn sh23_crypto_tls13_peer_crt_eval_smoke() {
    spawn_tls13_peer(28382, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_crt_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-crt".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf CN word cert");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf CN word cert");
            eprintln!("sh23 TLS 1.3 rustls peer leaf CN word cert value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf CN word rcgen vs rustls P-256 peer :28383.
#[test]
fn sh23_crypto_tls13_peer_rcg_eval_smoke() {
    spawn_tls13_peer(28383, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_rcg_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-rcg".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf CN word rcgen");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf CN word rcgen");
            eprintln!("sh23 TLS 1.3 rustls peer leaf CN word rcgen value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf single CN vs rustls P-256 peer :28384.
#[test]
fn sh23_crypto_tls13_peer_scn_eval_smoke() {
    spawn_tls13_peer(28384, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_scn_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-scn".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf single CN");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf single CN");
            eprintln!("sh23 TLS 1.3 rustls peer leaf single CN value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf subject one RDN vs rustls P-256 peer :28385.
#[test]
fn sh23_crypto_tls13_peer_rdn_eval_smoke() {
    spawn_tls13_peer(28385, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_rdn_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-rdn".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf one RDN");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf one RDN");
            eprintln!("sh23 TLS 1.3 rustls peer leaf one RDN value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf issuer one RDN vs rustls P-256 peer :28386.
#[test]
fn sh23_crypto_tls13_peer_ird_eval_smoke() {
    spawn_tls13_peer(28386, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_ird_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-ird".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf issuer one RDN");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf issuer one RDN");
            eprintln!("sh23 TLS 1.3 rustls peer leaf issuer one RDN value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 leaf issuer single CN vs rustls P-256 peer :28387.
#[test]
fn sh23_crypto_tls13_peer_icn_eval_smoke() {
    spawn_tls13_peer(28387, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_icn_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-icn".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer leaf issuer single CN");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer leaf issuer single CN");
            eprintln!("sh23 TLS 1.3 rustls peer leaf issuer single CN value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN no dNSName vs rustls P-256 peer :28388.
#[test]
fn sh23_crypto_tls13_peer_ndn_eval_smoke() {
    spawn_tls13_peer(28388, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_ndn_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-ndn".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN no dNSName");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN no dNSName");
            eprintln!("sh23 TLS 1.3 rustls peer SAN no dNSName value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN no rfc822Name vs rustls P-256 peer :28389.
#[test]
fn sh23_crypto_tls13_peer_nml_eval_smoke() {
    spawn_tls13_peer(28389, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_nml_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-nml".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN no rfc822Name");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN no rfc822Name");
            eprintln!("sh23 TLS 1.3 rustls peer SAN no rfc822Name value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN no URI vs rustls P-256 peer :28390.
#[test]
fn sh23_crypto_tls13_peer_nur_eval_smoke() {
    spawn_tls13_peer(28390, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_nur_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-nur".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN no URI");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN no URI");
            eprintln!("sh23 TLS 1.3 rustls peer SAN no URI value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN no otherName vs rustls P-256 peer :28391.
#[test]
fn sh23_crypto_tls13_peer_oth_eval_smoke() {
    spawn_tls13_peer(28391, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_oth_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-oth".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN no otherName");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN no otherName");
            eprintln!("sh23 TLS 1.3 rustls peer SAN no otherName value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN no x400Address vs rustls P-256 peer :28392.
#[test]
fn sh23_crypto_tls13_peer_x4a_eval_smoke() {
    spawn_tls13_peer(28392, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_x4a_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-x4a".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN no x400Address");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN no x400Address");
            eprintln!("sh23 TLS 1.3 rustls peer SAN no x400Address value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN no directoryName vs rustls P-256 peer :28393.
#[test]
fn sh23_crypto_tls13_peer_dnm_eval_smoke() {
    spawn_tls13_peer(28393, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_dnm_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-dnm".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN no directoryName");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN no directoryName");
            eprintln!("sh23 TLS 1.3 rustls peer SAN no directoryName value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN no ediPartyName vs rustls P-256 peer :28394.
#[test]
fn sh23_crypto_tls13_peer_epn_eval_smoke() {
    spawn_tls13_peer(28394, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_epn_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-epn".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN no ediPartyName");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN no ediPartyName");
            eprintln!("sh23 TLS 1.3 rustls peer SAN no ediPartyName value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN no registeredID vs rustls P-256 peer :28395.
#[test]
fn sh23_crypto_tls13_peer_rid_eval_smoke() {
    spawn_tls13_peer(28395, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_rid_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-rid".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN no registeredID");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN no registeredID");
            eprintln!("sh23 TLS 1.3 rustls peer SAN no registeredID value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress-only vs rustls P-256 peer :28396.
#[test]
fn sh23_crypto_tls13_peer_ipo_eval_smoke() {
    spawn_tls13_peer(28396, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_ipo_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-ipo".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress-only");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress-only");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress-only value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress IPv4 4 bytes vs rustls P-256 peer :28397.
#[test]
fn sh23_crypto_tls13_peer_ip4_eval_smoke() {
    spawn_tls13_peer(28397, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_ip4_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-ip4".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress IPv4 4 bytes");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress IPv4 4 bytes");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress IPv4 4 bytes value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress 127.0.0.1 vs rustls P-256 peer :28398.
#[test]
fn sh23_crypto_tls13_peer_lip_eval_smoke() {
    spawn_tls13_peer(28398, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_lip_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-lip".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress 127.0.0.1");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress 127.0.0.1");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress 127.0.0.1 value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress not multicast vs rustls P-256 peer :28399.
#[test]
fn sh23_crypto_tls13_peer_nmc_eval_smoke() {
    spawn_tls13_peer(28399, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_nmc_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-nmc".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress not multicast");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress not multicast");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress not multicast value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress not unspecified vs rustls P-256 peer :28400.
#[test]
fn sh23_crypto_tls13_peer_nus_eval_smoke() {
    spawn_tls13_peer(28400, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_nus_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-nus".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress not unspecified");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress not unspecified");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress not unspecified value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress not broadcast vs rustls P-256 peer :28401.
#[test]
fn sh23_crypto_tls13_peer_nbc_eval_smoke() {
    spawn_tls13_peer(28401, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_nbc_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-nbc".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress not broadcast");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress not broadcast");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress not broadcast value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress not link-local vs rustls P-256 peer :28402.
#[test]
fn sh23_crypto_tls13_peer_nll_eval_smoke() {
    spawn_tls13_peer(28402, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_nll_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-nll".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress not link-local");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress not link-local");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress not link-local value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress not RFC1918 10/8 vs rustls P-256 peer :28403.
#[test]
fn sh23_crypto_tls13_peer_n10_eval_smoke() {
    spawn_tls13_peer(28403, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n10_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n10".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress not RFC1918 10/8");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress not RFC1918 10/8");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress not RFC1918 10/8 value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress not RFC1918 172.16/12 vs rustls P-256 peer :28404.
#[test]
fn sh23_crypto_tls13_peer_n172_eval_smoke() {
    spawn_tls13_peer(28404, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n172_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n172".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress not RFC1918 172.16/12");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress not RFC1918 172.16/12");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress not RFC1918 172.16/12 value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress not RFC1918 192.168/16 vs rustls P-256 peer :28405.
#[test]
fn sh23_crypto_tls13_peer_n192_eval_smoke() {
    spawn_tls13_peer(28405, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n192_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n192".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress not RFC1918 192.168/16");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress not RFC1918 192.168/16");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress not RFC1918 192.168/16 value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress not RFC6598 100.64/10 vs rustls P-256 peer :28406.
#[test]
fn sh23_crypto_tls13_peer_n100_eval_smoke() {
    spawn_tls13_peer(28406, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n100_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n100".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress not RFC6598 100.64/10");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress not RFC6598 100.64/10");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress not RFC6598 100.64/10 value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress not RFC5737 TEST-NET-1 192.0.2/24 vs rustls P-256 peer :28407.
#[test]
fn sh23_crypto_tls13_peer_n202_eval_smoke() {
    spawn_tls13_peer(28407, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n202_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n202".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress not RFC5737 TEST-NET-1 192.0.2/24");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress not RFC5737 TEST-NET-1 192.0.2/24");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress not RFC5737 TEST-NET-1 192.0.2/24 value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress not RFC5737 TEST-NET-2 198.51.100/24 vs rustls P-256 peer :28408.
#[test]
fn sh23_crypto_tls13_peer_n198_eval_smoke() {
    spawn_tls13_peer(28408, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n198_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n198".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress not RFC5737 TEST-NET-2 198.51.100/24");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress not RFC5737 TEST-NET-2 198.51.100/24");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress not RFC5737 TEST-NET-2 198.51.100/24 value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress not RFC5737 TEST-NET-3 203.0.113/24 vs rustls P-256 peer :28409.
#[test]
fn sh23_crypto_tls13_peer_n203_eval_smoke() {
    spawn_tls13_peer(28409, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n203_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n203".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress not RFC5737 TEST-NET-3 203.0.113/24");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress not RFC5737 TEST-NET-3 203.0.113/24");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress not RFC5737 TEST-NET-3 203.0.113/24 value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress not RFC2544 198.18/15 vs rustls P-256 peer :28410.
#[test]
fn sh23_crypto_tls13_peer_n218_eval_smoke() {
    spawn_tls13_peer(28410, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n218_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n218".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress not RFC2544 198.18/15");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress not RFC2544 198.18/15");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress not RFC2544 198.18/15 value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress not RFC6890 IETF 192.0.0/24 vs rustls P-256 peer :28411.
#[test]
fn sh23_crypto_tls13_peer_n200_eval_smoke() {
    spawn_tls13_peer(28411, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n200_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n200".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress not RFC6890 IETF 192.0.0/24");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress not RFC6890 IETF 192.0.0/24");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress not RFC6890 IETF 192.0.0/24 value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress not RFC7526 192.88.99/24 vs rustls P-256 peer :28412.
#[test]
fn sh23_crypto_tls13_peer_n288_eval_smoke() {
    spawn_tls13_peer(28412, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n288_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n288".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress not RFC7526 192.88.99/24");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress not RFC7526 192.88.99/24");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress not RFC7526 192.88.99/24 value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress not RFC6890 reserved 240/4 vs rustls P-256 peer :28413.
#[test]
fn sh23_crypto_tls13_peer_n240_eval_smoke() {
    spawn_tls13_peer(28413, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n240_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n240".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress not RFC6890 reserved 240/4");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress not RFC6890 reserved 240/4");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress not RFC6890 reserved 240/4 value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress not RFC6890 this-network 0/8 vs rustls P-256 peer :28414.
#[test]
fn sh23_crypto_tls13_peer_n008_eval_smoke() {
    spawn_tls13_peer(28414, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n008_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n008".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress not RFC6890 this-network 0/8");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress not RFC6890 this-network 0/8");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress not RFC6890 this-network 0/8 value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress not RFC6890 AS112 192.31.196/24 vs rustls P-256 peer :28415.
#[test]
fn sh23_crypto_tls13_peer_n231_eval_smoke() {
    spawn_tls13_peer(28415, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n231_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n231".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress not RFC6890 AS112 192.31.196/24");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress not RFC6890 AS112 192.31.196/24");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress not RFC6890 AS112 192.31.196/24 value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress not RFC6890 AMT 192.52.193/24 vs rustls P-256 peer :28426.
#[test]
fn sh23_crypto_tls13_peer_n252_eval_smoke() {
    spawn_tls13_peer(28426, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n252_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n252".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress not RFC6890 AMT 192.52.193/24");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress not RFC6890 AMT 192.52.193/24");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress not RFC6890 AMT 192.52.193/24 value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress not RFC7535 AS112 192.175.48/24 vs rustls P-256 peer :28427.
#[test]
fn sh23_crypto_tls13_peer_n175_eval_smoke() {
    spawn_tls13_peer(28427, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n175_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n175".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress not RFC7535 AS112 192.175.48/24");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress not RFC7535 AS112 192.175.48/24");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress not RFC7535 AS112 192.175.48/24 value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress not RFC6890 255/8 vs rustls P-256 peer :28438.
#[test]
fn sh23_crypto_tls13_peer_n255_eval_smoke() {
    spawn_tls13_peer(28438, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n255_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n255".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress not RFC6890 255/8");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress not RFC6890 255/8");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress not RFC6890 255/8 value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN otherName (tag 160) rejected vs rustls P-256 peer :28439.
#[test]
fn sh23_crypto_tls13_peer_n160_eval_smoke() {
    spawn_tls13_peer(28439, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n160_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n160".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN otherName (tag 160) rejected");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN otherName (tag 160) rejected");
            eprintln!("sh23 TLS 1.3 rustls peer SAN otherName (tag 160) rejected value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN rfc822Name (tag 129) rejected vs rustls P-256 peer :28440.
#[test]
fn sh23_crypto_tls13_peer_n129_eval_smoke() {
    spawn_tls13_peer(28440, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n129_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n129".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN rfc822Name (tag 129) rejected");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN rfc822Name (tag 129) rejected");
            eprintln!("sh23 TLS 1.3 rustls peer SAN rfc822Name (tag 129) rejected value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN dNSName (tag 130) rejected vs rustls P-256 peer :28441.
#[test]
fn sh23_crypto_tls13_peer_n130_eval_smoke() {
    spawn_tls13_peer(28441, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n130_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n130".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN dNSName (tag 130) rejected");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN dNSName (tag 130) rejected");
            eprintln!("sh23 TLS 1.3 rustls peer SAN dNSName (tag 130) rejected value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN x400Address (tag 131) rejected vs rustls P-256 peer :28442.
#[test]
fn sh23_crypto_tls13_peer_n131_eval_smoke() {
    spawn_tls13_peer(28442, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n131_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n131".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN x400Address (tag 131) rejected");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN x400Address (tag 131) rejected");
            eprintln!("sh23 TLS 1.3 rustls peer SAN x400Address (tag 131) rejected value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN directoryName (tag 132) rejected vs rustls P-256 peer :28443.
#[test]
fn sh23_crypto_tls13_peer_n132_eval_smoke() {
    spawn_tls13_peer(28443, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n132_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n132".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN directoryName (tag 132) rejected");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN directoryName (tag 132) rejected");
            eprintln!("sh23 TLS 1.3 rustls peer SAN directoryName (tag 132) rejected value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN ediPartyName (tag 133) rejected vs rustls P-256 peer :28444.
#[test]
fn sh23_crypto_tls13_peer_n133_eval_smoke() {
    spawn_tls13_peer(28444, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n133_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n133".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN ediPartyName (tag 133) rejected");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN ediPartyName (tag 133) rejected");
            eprintln!("sh23 TLS 1.3 rustls peer SAN ediPartyName (tag 133) rejected value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN uniformResourceIdentifier (tag 134) rejected vs rustls P-256 peer :28445.
#[test]
fn sh23_crypto_tls13_peer_n134_eval_smoke() {
    spawn_tls13_peer(28445, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n134_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n134".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN uniformResourceIdentifier (tag 134) rejected");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN uniformResourceIdentifier (tag 134) rejected");
            eprintln!("sh23 TLS 1.3 rustls peer SAN uniformResourceIdentifier (tag 134) rejected value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN registeredID (tag 136) rejected vs rustls P-256 peer :28446.
#[test]
fn sh23_crypto_tls13_peer_n136_eval_smoke() {
    spawn_tls13_peer(28446, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n136_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n136".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN registeredID (tag 136) rejected");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN registeredID (tag 136) rejected");
            eprintln!("sh23 TLS 1.3 rustls peer SAN registeredID (tag 136) rejected value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress IPv6 (16 byte) rejected vs rustls P-256 peer :28447.
#[test]
fn sh23_crypto_tls13_peer_n16_eval_smoke() {
    spawn_tls13_peer(28447, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n16_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n16".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress IPv6 (16 byte) rejected");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress IPv6 (16 byte) rejected");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress IPv6 (16 byte) rejected value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress empty (len 0) rejected vs rustls P-256 peer :28448.
#[test]
fn sh23_crypto_tls13_peer_n0_eval_smoke() {
    spawn_tls13_peer(28448, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n0_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n0".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress empty (len 0) rejected");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress empty (len 0) rejected");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress empty (len 0) rejected value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress length 1 rejected vs rustls P-256 peer :28449.
#[test]
fn sh23_crypto_tls13_peer_n1_eval_smoke() {
    spawn_tls13_peer(28449, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n1_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n1".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress length 1 rejected");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress length 1 rejected");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress length 1 rejected value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress length 2 rejected vs rustls P-256 peer :28450.
#[test]
fn sh23_crypto_tls13_peer_n2_eval_smoke() {
    spawn_tls13_peer(28450, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n2_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n2".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress length 2 rejected");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress length 2 rejected");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress length 2 rejected value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress length 3 rejected vs rustls P-256 peer :28451.
#[test]
fn sh23_crypto_tls13_peer_n3_eval_smoke() {
    spawn_tls13_peer(28451, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n3_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n3".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress length 3 rejected");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress length 3 rejected");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress length 3 rejected value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress length 5 rejected vs rustls P-256 peer :28452.
#[test]
fn sh23_crypto_tls13_peer_n5_eval_smoke() {
    spawn_tls13_peer(28452, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n5_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n5".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress length 5 rejected");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress length 5 rejected");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress length 5 rejected value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress length 6 rejected vs rustls P-256 peer :28453.
#[test]
fn sh23_crypto_tls13_peer_n6_eval_smoke() {
    spawn_tls13_peer(28453, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n6_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n6".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress length 6 rejected");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress length 6 rejected");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress length 6 rejected value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress length 7 rejected vs rustls P-256 peer :28454.
#[test]
fn sh23_crypto_tls13_peer_n7_eval_smoke() {
    spawn_tls13_peer(28454, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n7_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n7".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress length 7 rejected");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress length 7 rejected");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress length 7 rejected value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress length 8 rejected vs rustls P-256 peer :28455.
#[test]
fn sh23_crypto_tls13_peer_n8_eval_smoke() {
    spawn_tls13_peer(28455, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n8_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n8".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress length 8 rejected");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress length 8 rejected");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress length 8 rejected value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress length 9 rejected vs rustls P-256 peer :28456.
#[test]
fn sh23_crypto_tls13_peer_n9_eval_smoke() {
    spawn_tls13_peer(28456, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n9_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n9".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress length 9 rejected");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress length 9 rejected");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress length 9 rejected value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}

/// SH23: Kab TLS 1.3 SAN iPAddress length 11 rejected vs rustls P-256 peer :28457.
#[test]
fn sh23_crypto_tls13_peer_n11_eval_smoke() {
    spawn_tls13_peer(28457, true, false);
    std::thread::sleep(Duration::from_millis(80));
    let path = format!(
        "{}/examples/sh23_crypto_tls13_peer_n11_eval_smoke.kab",
        env!("CARGO_MANIFEST_DIR")
    );
    std::thread::Builder::new()
        .name("sh23-crypto-tls13-peer-n11".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            use kabootar_lib::compile::{compile_file_cached, eval_program};
            let mut env = create_global_env();
            let program = compile_file_cached(&path).expect("compile TLS 1.3 rustls peer SAN iPAddress length 11 rejected");
            let value = eval_program(&program, &mut env).expect("run TLS 1.3 rustls peer SAN iPAddress length 11 rejected");
            eprintln!("sh23 TLS 1.3 rustls peer SAN iPAddress length 11 rejected value = {value:?}");
            assert!(matches!(value, kabootar_lib::value::Value::Bool(true)));
        })
        .expect("spawn")
        .join()
        .expect("join");
}
