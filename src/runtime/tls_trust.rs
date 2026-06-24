//! TLS trust configuration — custom CA and certificate pinning (v2.11).

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TlsTrust {
    pub extra_ca_der: Vec<Vec<u8>>,
    pub pins: HashMap<String, String>,
    pub mozilla_roots: bool,
}

impl Default for TlsTrust {
    fn default() -> Self {
        Self {
            extra_ca_der: Vec::new(),
            pins: HashMap::new(),
            mozilla_roots: true,
        }
    }
}

impl TlsTrust {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn add_ca_pem(&mut self, pem: &str) -> Result<(), String> {
        for der in parse_pem_certs(pem)? {
            self.extra_ca_der.push(der);
        }
        Ok(())
    }

    pub fn set_ca_only_pem(&mut self, pem: &str) -> Result<(), String> {
        let certs = parse_pem_certs(pem)?;
        if certs.is_empty() {
            return Err("tls_ca_only() found no certificates in PEM".into());
        }
        self.mozilla_roots = false;
        self.extra_ca_der = certs;
        Ok(())
    }

    pub fn pin_host(&mut self, host: &str, sha256_hex: &str) -> Result<(), String> {
        let normalized = normalize_fingerprint(sha256_hex)?;
        self.pins.insert(host.to_ascii_lowercase(), normalized);
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn cert_pem_sha256_hex(pem: &str) -> Result<String, String> {
    use sha2::{Digest, Sha256};

    let certs = parse_pem_certs(pem)?;
    let first = certs
        .first()
        .ok_or("No certificate found in PEM")?;
    let hash = Sha256::digest(first);
    Ok(hex_encode(&hash))
}

#[cfg(target_arch = "wasm32")]
pub fn cert_pem_sha256_hex(_pem: &str) -> Result<String, String> {
    Err("tls_cert_sha256 requires native runtime".into())
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_pem_certs(pem: &str) -> Result<Vec<Vec<u8>>, String> {
    use rustls_pemfile::certs;
    use std::io::BufReader;

    let mut reader = BufReader::new(pem.as_bytes());
    let items: Vec<_> = certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to parse PEM certificate: {e}"))?;
    if items.is_empty() {
        return Err("No certificates found in PEM".into());
    }
    Ok(items.into_iter().map(|c| c.to_vec()).collect())
}

#[cfg(target_arch = "wasm32")]
fn parse_pem_certs(_pem: &str) -> Result<Vec<Vec<u8>>, String> {
    Err("PEM parsing requires native runtime".into())
}

fn normalize_fingerprint(hex: &str) -> Result<String, String> {
    let cleaned: String = hex
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ':')
        .collect::<String>()
        .to_ascii_lowercase();
    if cleaned.len() != 64 || !cleaned.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Certificate pin must be 64 hex characters (SHA-256)".into());
    }
    Ok(cleaned)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn verify_peer_pin(host: &str, peer_cert_der: &[u8], trust: &TlsTrust) -> Result<(), String> {
    use sha2::{Digest, Sha256};

    let key = host.to_ascii_lowercase();
    let Some(expected) = trust.pins.get(&key) else {
        return Ok(());
    };
    let actual = hex_encode(&Sha256::digest(peer_cert_der));
    if &actual != expected {
        return Err(format!(
            "Certificate pin mismatch for {host}: expected {expected}, got {actual}"
        ));
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub fn verify_peer_pin(_host: &str, _peer_cert_der: &[u8], _trust: &TlsTrust) -> Result<(), String> {
    Ok(())
}

use crate::value::{Environment, Value};

fn expect_string(args: &[Value], index: usize, name: &str) -> Result<String, String> {
    match args.get(index) {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(format!("{name} expects a string argument")),
    }
}

fn tls_add_ca_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let pem = expect_string(args, 0, "tls_add_ca()")?;
    env.tls_trust_mut().add_ca_pem(&pem)?;
    Ok(Value::Null)
}

fn tls_ca_only_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let pem = expect_string(args, 0, "tls_ca_only()")?;
    env.tls_trust_mut().set_ca_only_pem(&pem)?;
    Ok(Value::Null)
}

fn tls_pin_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let host = expect_string(args, 0, "tls_pin()")?;
    let sha256_hex = expect_string(args, 1, "tls_pin()")?;
    env.tls_trust_mut().pin_host(&host, &sha256_hex)?;
    Ok(Value::Null)
}

fn tls_reset_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    env.tls_trust_mut().reset();
    Ok(Value::Null)
}

fn tls_cert_sha256_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let pem = expect_string(args, 0, "tls_cert_sha256()")?;
    Ok(Value::String(cert_pem_sha256_hex(&pem)?))
}

pub fn tls_trust_globals(env: &mut Environment) {
    env.set(
        "tls_add_ca".to_string(),
        Value::NativeFunction(tls_add_ca_native),
    );
    env.set(
        "tls_ca_only".to_string(),
        Value::NativeFunction(tls_ca_only_native),
    );
    env.set(
        "tls_pin".to_string(),
        Value::NativeFunction(tls_pin_native),
    );
    env.set(
        "tls_reset".to_string(),
        Value::NativeFunction(tls_reset_native),
    );
    env.set(
        "tls_cert_sha256".to_string(),
        Value::NativeFunction(tls_cert_sha256_native),
    );
}
