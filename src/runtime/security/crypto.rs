use super::secure_bytes::SecureBytes;
use crate::value::Value;

#[cfg(feature = "crypto")]
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
#[cfg(feature = "crypto")]
use argon2::{Algorithm, Argon2, Params, Version};
#[cfg(feature = "crypto")]
use chacha20poly1305::ChaCha20Poly1305;
#[cfg(feature = "crypto")]
use p256::ecdsa::{signature::Signer, signature::Verifier, Signature, SigningKey, VerifyingKey};
#[cfg(feature = "crypto")]
use rand::RngCore;
#[cfg(feature = "crypto")]
use rsa::pkcs1::{DecodeRsaPublicKey, EncodeRsaPublicKey};
#[cfg(feature = "crypto")]
use rsa::pkcs8::{DecodePrivateKey, EncodePrivateKey, EncodePublicKey};
#[cfg(feature = "crypto")]
use rsa::{pkcs1v15::Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
#[cfg(feature = "crypto")]
use sha3::{Digest, Sha3_256, Sha3_512};

const CRYPTO_DISABLED: &str =
    "Crypto not enabled. Rebuild Kabootar with: cargo build --features crypto";

pub fn value_to_bytes(v: &Value) -> Result<Vec<u8>, String> {
    match v {
        Value::Array(items) => items
            .iter()
            .map(|item| match item {
                Value::Number(n) if (0..=255).contains(n) => Ok(*n as u8),
                _ => Err("byte array must contain integers 0-255".into()),
            })
            .collect(),
        Value::String(s) => Ok(s.as_bytes().to_vec()),
        Value::SecureBytes(buf) => buf.read(),
        _ => Err("expected byte array, string, or secure buffer".into()),
    }
}

pub fn bytes_to_array(bytes: &[u8]) -> Value {
    Value::Array(
        bytes
            .iter()
            .map(|b| Value::Number(*b as i64))
            .collect(),
    )
}

pub fn secure_from_bytes(bytes: Vec<u8>) -> Value {
    Value::SecureBytes(SecureBytes::from_vec(bytes))
}

#[cfg(feature = "crypto")]
pub fn random_bytes(len: usize) -> Result<Vec<u8>, String> {
    let len = len.clamp(1, 65536);
    let mut buf = vec![0u8; len];
    rand::thread_rng().fill_bytes(&mut buf);
    Ok(buf)
}

#[cfg(not(feature = "crypto"))]
pub fn random_bytes(_len: usize) -> Result<Vec<u8>, String> {
    Err(CRYPTO_DISABLED.into())
}

#[cfg(feature = "crypto")]
pub fn sha3_256(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha3_256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

#[cfg(not(feature = "crypto"))]
pub fn sha3_256(_data: &[u8]) -> Vec<u8> {
    Vec::new()
}

#[cfg(feature = "crypto")]
pub fn sha3_512(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha3_512::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

#[cfg(not(feature = "crypto"))]
pub fn sha3_512(_data: &[u8]) -> Vec<u8> {
    Vec::new()
}

pub fn sha3_256_checked(data: &[u8]) -> Result<Vec<u8>, String> {
    #[cfg(feature = "crypto")]
    {
        return Ok(sha3_256(data));
    }
    #[cfg(not(feature = "crypto"))]
    {
        let _ = data;
        Err(CRYPTO_DISABLED.into())
    }
}

pub fn sha3_512_checked(data: &[u8]) -> Result<Vec<u8>, String> {
    #[cfg(feature = "crypto")]
    {
        return Ok(sha3_512(data));
    }
    #[cfg(not(feature = "crypto"))]
    {
        let _ = data;
        Err(CRYPTO_DISABLED.into())
    }
}

#[cfg(feature = "crypto")]
pub fn argon2_hash(password: &[u8], salt: &[u8], m_kb: u32, t: u32, p: u32) -> Result<Vec<u8>, String> {
    if salt.len() < 8 {
        return Err("Argon2 salt must be at least 8 bytes".into());
    }
    let params = Params::new(m_kb, t, p, Some(32))
        .map_err(|e| format!("Argon2 params: {}", e))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = vec![0u8; 32];
    argon
        .hash_password_into(password, salt, &mut out)
        .map_err(|e| format!("Argon2: {}", e))?;
    Ok(out)
}

#[cfg(not(feature = "crypto"))]
pub fn argon2_hash(
    _password: &[u8],
    _salt: &[u8],
    _m_kb: u32,
    _t: u32,
    _p: u32,
) -> Result<Vec<u8>, String> {
    Err(CRYPTO_DISABLED.into())
}

#[cfg(feature = "crypto")]
pub fn aes256_encrypt(key: &[u8], nonce: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    if key.len() != 32 {
        return Err("AES-256 requires a 32-byte key".into());
    }
    if nonce.len() != 12 {
        return Err("AES-GCM requires a 12-byte nonce".into());
    }
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("AES key: {}", e))?;
    let nonce = Nonce::from_slice(nonce);
    cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| format!("AES encrypt: {}", e))
}

#[cfg(not(feature = "crypto"))]
pub fn aes256_encrypt(_key: &[u8], _nonce: &[u8], _plaintext: &[u8]) -> Result<Vec<u8>, String> {
    Err(CRYPTO_DISABLED.into())
}

#[cfg(feature = "crypto")]
pub fn aes256_decrypt(key: &[u8], nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    if key.len() != 32 {
        return Err("AES-256 requires a 32-byte key".into());
    }
    if nonce.len() != 12 {
        return Err("AES-GCM requires a 12-byte nonce".into());
    }
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("AES key: {}", e))?;
    let nonce = Nonce::from_slice(nonce);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("AES decrypt: {}", e))
}

#[cfg(not(feature = "crypto"))]
pub fn aes256_decrypt(_key: &[u8], _nonce: &[u8], _ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    Err(CRYPTO_DISABLED.into())
}

#[cfg(feature = "crypto")]
pub fn chacha20_encrypt(key: &[u8], nonce: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    if key.len() != 32 {
        return Err("ChaCha20-Poly1305 requires a 32-byte key".into());
    }
    if nonce.len() != 12 {
        return Err("ChaCha20-Poly1305 requires a 12-byte nonce".into());
    }
    let cipher =
        ChaCha20Poly1305::new_from_slice(key).map_err(|e| format!("ChaCha key: {}", e))?;
    let nonce = chacha20poly1305::Nonce::from_slice(nonce);
    cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| format!("ChaCha encrypt: {}", e))
}

#[cfg(not(feature = "crypto"))]
pub fn chacha20_encrypt(_key: &[u8], _nonce: &[u8], _plaintext: &[u8]) -> Result<Vec<u8>, String> {
    Err(CRYPTO_DISABLED.into())
}

#[cfg(feature = "crypto")]
pub fn chacha20_decrypt(key: &[u8], nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    if key.len() != 32 {
        return Err("ChaCha20-Poly1305 requires a 32-byte key".into());
    }
    if nonce.len() != 12 {
        return Err("ChaCha20-Poly1305 requires a 12-byte nonce".into());
    }
    let cipher =
        ChaCha20Poly1305::new_from_slice(key).map_err(|e| format!("ChaCha key: {}", e))?;
    let nonce = chacha20poly1305::Nonce::from_slice(nonce);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("ChaCha decrypt: {}", e))
}

#[cfg(not(feature = "crypto"))]
pub fn chacha20_decrypt(_key: &[u8], _nonce: &[u8], _ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    Err(CRYPTO_DISABLED.into())
}

#[cfg(feature = "crypto")]
pub fn rsa_generate(bits: usize) -> Result<(Vec<u8>, Vec<u8>), String> {
    let bits = bits.clamp(2048, 4096);
    let mut rng = rand::thread_rng();
    let private = RsaPrivateKey::new(&mut rng, bits).map_err(|e| format!("RSA generate: {}", e))?;
    let public = RsaPublicKey::from(&private);
    let priv_der = private
        .to_pkcs8_der()
        .map_err(|e| format!("RSA private DER: {}", e))?
        .as_bytes()
        .to_vec();
    let pub_der = public
        .to_pkcs1_der()
        .map_err(|e| format!("RSA public DER: {}", e))?
        .as_bytes()
        .to_vec();
    Ok((pub_der, priv_der))
}

#[cfg(not(feature = "crypto"))]
pub fn rsa_generate(_bits: usize) -> Result<(Vec<u8>, Vec<u8>), String> {
    Err(CRYPTO_DISABLED.into())
}

#[cfg(feature = "crypto")]
pub fn rsa_encrypt(public_key: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let public = RsaPublicKey::from_pkcs1_der(public_key)
        .map_err(|e| format!("RSA public key: {}", e))?;
    let mut rng = rand::thread_rng();
    public
        .encrypt(&mut rng, Pkcs1v15Encrypt, plaintext)
        .map_err(|e| format!("RSA encrypt: {}", e))
}

#[cfg(not(feature = "crypto"))]
pub fn rsa_encrypt(_public_key: &[u8], _plaintext: &[u8]) -> Result<Vec<u8>, String> {
    Err(CRYPTO_DISABLED.into())
}

#[cfg(feature = "crypto")]
pub fn rsa_decrypt(private_key: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    let private = RsaPrivateKey::from_pkcs8_der(private_key)
        .map_err(|e| format!("RSA private key: {}", e))?;
    private
        .decrypt(Pkcs1v15Encrypt, ciphertext)
        .map_err(|e| format!("RSA decrypt: {}", e))
}

#[cfg(not(feature = "crypto"))]
pub fn rsa_decrypt(_private_key: &[u8], _ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    Err(CRYPTO_DISABLED.into())
}

#[cfg(feature = "crypto")]
pub fn ecc_generate() -> Result<(Vec<u8>, Vec<u8>), String> {
    let mut rng = rand::thread_rng();
    let signing = SigningKey::random(&mut rng);
    let verifying = VerifyingKey::from(&signing);
    Ok((
        verifying.to_encoded_point(true).as_bytes().to_vec(),
        signing.to_bytes().to_vec(),
    ))
}

#[cfg(not(feature = "crypto"))]
pub fn ecc_generate() -> Result<(Vec<u8>, Vec<u8>), String> {
    Err(CRYPTO_DISABLED.into())
}

#[cfg(feature = "crypto")]
pub fn ecc_sign(private_key: &[u8], message: &[u8]) -> Result<Vec<u8>, String> {
    if private_key.len() != 32 {
        return Err("ECC P-256 private key must be 32 bytes".into());
    }
    let signing = SigningKey::from_bytes(private_key.into())
        .map_err(|e| format!("ECC private key: {}", e))?;
    let sig: Signature = signing.sign(message);
    Ok(sig.to_bytes().to_vec())
}

#[cfg(not(feature = "crypto"))]
pub fn ecc_sign(_private_key: &[u8], _message: &[u8]) -> Result<Vec<u8>, String> {
    Err(CRYPTO_DISABLED.into())
}

#[cfg(feature = "crypto")]
pub fn ecc_verify(public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<bool, String> {
    let verifying = VerifyingKey::from_sec1_bytes(public_key)
        .map_err(|e| format!("ECC public key: {}", e))?;
    let sig = Signature::from_slice(signature).map_err(|e| format!("ECC signature: {}", e))?;
    Ok(verifying.verify(message, &sig).is_ok())
}

#[cfg(not(feature = "crypto"))]
pub fn ecc_verify(_public_key: &[u8], _message: &[u8], _signature: &[u8]) -> Result<bool, String> {
    Err(CRYPTO_DISABLED.into())
}

/// CRYSTALS-Kyber768 stub — deterministic KEM wiring for post-quantum paths (not FIPS).
#[cfg(feature = "crypto")]
pub fn kyber768_encapsulate(public_key: &[u8]) -> Result<(Vec<u8>, Vec<u8>), String> {
    if public_key.is_empty() {
        return Err("kyber768_encapsulate expects public key bytes".into());
    }
    let ciphertext = sha3_256(public_key);
    let mut shared = sha3_512(public_key);
    shared.truncate(32);
    Ok((ciphertext, shared))
}

#[cfg(not(feature = "crypto"))]
pub fn kyber768_encapsulate(_public_key: &[u8]) -> Result<(Vec<u8>, Vec<u8>), String> {
    Err(CRYPTO_DISABLED.into())
}

/// CRYSTALS-Dilithium stub — deterministic signature wiring (not FIPS).
#[cfg(feature = "crypto")]
pub fn dilithium_sign_stub(message: &[u8], secret_seed: &[u8]) -> Result<Vec<u8>, String> {
    if secret_seed.is_empty() {
        return Err("dilithium_sign_stub expects secret seed bytes".into());
    }
    let mut data = secret_seed.to_vec();
    data.extend_from_slice(message);
    Ok(sha3_512(&data))
}

#[cfg(not(feature = "crypto"))]
pub fn dilithium_sign_stub(_message: &[u8], _secret_seed: &[u8]) -> Result<Vec<u8>, String> {
    Err(CRYPTO_DISABLED.into())
}

#[cfg(feature = "crypto")]
mod tests {
    use super::*;

    #[test]
    fn aes_roundtrip() {
        let key = vec![7u8; 32];
        let nonce = vec![1u8; 12];
        let plain = b"kabootar";
        let enc = aes256_encrypt(&key, &nonce, plain).unwrap();
        let dec = aes256_decrypt(&key, &nonce, &enc).unwrap();
        assert_eq!(dec, plain);
    }

    #[test]
    fn sha3_256_len() {
        assert_eq!(sha3_256(b"test").len(), 32);
    }
}
