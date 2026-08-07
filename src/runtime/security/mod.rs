//! Kabootar security toolbox — primitives and pluggable backends, not policy.
//!
//! Kabootar gives developers weapons (AES, ChaCha20, RSA, ECC, SHA-3, Argon2),
//! secure memory wiping, device APIs, and swappable security providers.
//! **Policy** (ZK vs cloud, HSM vs YubiKey, quorum rules) belongs in application code.

mod crypto;
mod devices;
mod providers;
mod secure_bytes;

pub use devices::{DeviceHandle, DeviceKind};
pub use providers::ProviderId;
pub use secure_bytes::SecureBytes;
pub use crypto::random_bytes;

use crate::value::{Environment, Value};
use crypto::{
    aes256_decrypt, aes256_encrypt, argon2_hash, bytes_to_array, chacha20_decrypt,
    chacha20_encrypt, ecc_generate, ecc_sign, ecc_verify, dilithium_sign_stub, kyber768_encapsulate,
    rsa_decrypt,
    rsa_encrypt, rsa_generate, secure_from_bytes, value_to_bytes,
};
use devices::DeviceRegistry;
use providers::{device_kind_label, ProviderRegistry};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SecurityHandle {
    pub providers: Arc<Mutex<ProviderRegistry>>,
    pub devices: Arc<Mutex<DeviceRegistry>>,
}

impl std::fmt::Debug for SecurityHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecurityHandle").finish_non_exhaustive()
    }
}

impl SecurityHandle {
    pub fn new() -> Self {
        Self {
            providers: Arc::new(Mutex::new(ProviderRegistry::default())),
            devices: Arc::new(Mutex::new(DeviceRegistry::default())),
        }
    }

    fn with_security<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&mut ProviderRegistry, &mut DeviceRegistry) -> Result<T, String>,
    {
        let mut providers = self
            .providers
            .lock()
            .map_err(|_| "Security provider lock poisoned".to_string())?;
        let mut devices = self
            .devices
            .lock()
            .map_err(|_| "Device registry lock poisoned".to_string())?;
        f(&mut providers, &mut devices)
    }

    fn with_devices<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&mut DeviceRegistry) -> Result<T, String>,
    {
        let mut devices = self
            .devices
            .lock()
            .map_err(|_| "Device registry lock poisoned".to_string())?;
        f(&mut devices)
    }
}

fn get_security(env: &Environment) -> Result<SecurityHandle, String> {
    let sec = env.get("security").ok_or("Security handle not available")?;
    let Value::SecurityHandle(handle) = sec else {
        return Err("Security handle not available".into());
    };
    Ok(handle)
}

fn expect_usize(v: &Value, index: usize, name: &str) -> Result<usize, String> {
    match v {
        Value::Number(n) if *n >= 0 => Ok(*n as usize),
        _ => Err(format!("{} expects a non-negative integer at position {}", name, index)),
    }
}

fn crypto_random_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let len = match args.first() {
        Some(v) => expect_usize(v, 0, "crypto_random")?,
        None => 32,
    };
    let sec = get_security(env)?;
    let bytes = sec.with_security(|providers, devices| providers.random_bytes(devices, len))?;
    Ok(bytes_to_array(&bytes))
}

fn crypto_sha3_256_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let data = value_to_bytes(args.first().ok_or("crypto_sha3_256() expects data")?)?;
    Ok(bytes_to_array(&crypto::sha3_256_checked(&data)?))
}

fn crypto_sha3_512_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let data = value_to_bytes(args.first().ok_or("crypto_sha3_512() expects data")?)?;
    Ok(bytes_to_array(&crypto::sha3_512_checked(&data)?))
}

fn crypto_argon2_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let password = value_to_bytes(args.first().ok_or("crypto_argon2() expects password")?)?;
    let salt = value_to_bytes(args.get(1).ok_or("crypto_argon2() expects salt")?)?;
    let m_kb = args
        .get(2)
        .map(|v| expect_usize(v, 2, "crypto_argon2"))
        .transpose()?
        .unwrap_or(19456) as u32;
    let t = args
        .get(3)
        .map(|v| expect_usize(v, 3, "crypto_argon2"))
        .transpose()?
        .unwrap_or(2) as u32;
    let p = args
        .get(4)
        .map(|v| expect_usize(v, 4, "crypto_argon2"))
        .transpose()?
        .unwrap_or(1) as u32;
    Ok(bytes_to_array(&argon2_hash(&password, &salt, m_kb, t, p)?))
}

fn crypto_aes256_encrypt_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let key = value_to_bytes(args.first().ok_or("crypto_aes256_encrypt() expects key")?)?;
    let nonce = value_to_bytes(args.get(1).ok_or("crypto_aes256_encrypt() expects nonce")?)?;
    let plain = value_to_bytes(args.get(2).ok_or("crypto_aes256_encrypt() expects plaintext")?)?;
    Ok(bytes_to_array(&aes256_encrypt(&key, &nonce, &plain)?))
}

fn crypto_aes256_decrypt_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let key = value_to_bytes(args.first().ok_or("crypto_aes256_decrypt() expects key")?)?;
    let nonce = value_to_bytes(args.get(1).ok_or("crypto_aes256_decrypt() expects nonce")?)?;
    let cipher = value_to_bytes(args.get(2).ok_or("crypto_aes256_decrypt() expects ciphertext")?)?;
    Ok(bytes_to_array(&aes256_decrypt(&key, &nonce, &cipher)?))
}

fn crypto_chacha20_encrypt_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let key = value_to_bytes(args.first().ok_or("crypto_chacha20_encrypt() expects key")?)?;
    let nonce = value_to_bytes(args.get(1).ok_or("crypto_chacha20_encrypt() expects nonce")?)?;
    let plain = value_to_bytes(args.get(2).ok_or("crypto_chacha20_encrypt() expects plaintext")?)?;
    Ok(bytes_to_array(&chacha20_encrypt(&key, &nonce, &plain)?))
}

fn crypto_chacha20_decrypt_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let key = value_to_bytes(args.first().ok_or("crypto_chacha20_decrypt() expects key")?)?;
    let nonce = value_to_bytes(args.get(1).ok_or("crypto_chacha20_decrypt() expects nonce")?)?;
    let cipher =
        value_to_bytes(args.get(2).ok_or("crypto_chacha20_decrypt() expects ciphertext")?)?;
    Ok(bytes_to_array(&chacha20_decrypt(&key, &nonce, &cipher)?))
}

fn crypto_rsa_generate_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let bits = args
        .first()
        .map(|v| expect_usize(v, 0, "crypto_rsa_generate"))
        .transpose()?
        .unwrap_or(2048);
    let (public, private) = rsa_generate(bits)?;
    Ok(Value::from_array(vec![
        secure_from_bytes(public),
        secure_from_bytes(private),
    ]))
}

fn crypto_rsa_encrypt_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let public = value_to_bytes(args.first().ok_or("crypto_rsa_encrypt() expects public key")?)?;
    let plain = value_to_bytes(args.get(1).ok_or("crypto_rsa_encrypt() expects plaintext")?)?;
    Ok(bytes_to_array(&rsa_encrypt(&public, &plain)?))
}

fn crypto_rsa_decrypt_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let private =
        value_to_bytes(args.first().ok_or("crypto_rsa_decrypt() expects private key")?)?;
    let cipher = value_to_bytes(args.get(1).ok_or("crypto_rsa_decrypt() expects ciphertext")?)?;
    Ok(secure_from_bytes(rsa_decrypt(&private, &cipher)?))
}

fn crypto_ecc_generate_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let _ = args;
    let (public, private) = ecc_generate()?;
    Ok(Value::from_array(vec![
        bytes_to_array(&public),
        secure_from_bytes(private),
    ]))
}

fn crypto_ecc_sign_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let private = value_to_bytes(args.first().ok_or("crypto_ecc_sign() expects private key")?)?;
    let message = value_to_bytes(args.get(1).ok_or("crypto_ecc_sign() expects message")?)?;
    Ok(bytes_to_array(&ecc_sign(&private, &message)?))
}

fn crypto_ecc_verify_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let public = value_to_bytes(args.first().ok_or("crypto_ecc_verify() expects public key")?)?;
    let message = value_to_bytes(args.get(1).ok_or("crypto_ecc_verify() expects message")?)?;
    let signature = value_to_bytes(args.get(2).ok_or("crypto_ecc_verify() expects signature")?)?;
    Ok(Value::Bool(ecc_verify(&public, &message, &signature)?))
}

fn crypto_dilithium_sign_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let message = value_to_bytes(args.first().ok_or("crypto_dilithium_sign() expects message")?)?;
    let seed = value_to_bytes(args.get(1).ok_or("crypto_dilithium_sign() expects seed")?)?;
    Ok(bytes_to_array(&dilithium_sign_stub(&message, &seed)?))
}

fn crypto_kyber_encapsulate_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let public = value_to_bytes(args.first().ok_or("crypto_kyber_encapsulate() expects public key")?)?;
    let (ct, ss) = kyber768_encapsulate(&public)?;
    let mut o = std::collections::HashMap::new();
    o.insert("ciphertext".to_string(), bytes_to_array(&ct));
    o.insert("shared_secret".to_string(), bytes_to_array(&ss));
    o.insert("algorithm".to_string(), Value::String("CRYSTALS-Kyber768-stub".into()));
    Ok(Value::from_object(o))
}

fn crypto_secure_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let data = value_to_bytes(args.first().ok_or("crypto_secure() expects data")?)?;
    Ok(secure_from_bytes(data))
}

fn crypto_wipe_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let val = args.first().ok_or("crypto_wipe() expects a secure buffer")?;
    let Value::SecureBytes(buf) = val else {
        return Err("crypto_wipe() expects a secure buffer from crypto_secure()".into());
    };
    buf.wipe()?;
    Ok(Value::Null)
}

fn crypto_is_secure_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::Bool(matches!(args.first(), Some(Value::SecureBytes(_)))))
}

fn security_list_providers_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let _ = args;
    let items: Vec<Value> = ProviderId::all()
        .iter()
        .map(|id| {
            Value::from_array(vec![
                Value::String(id.name().into()),
                Value::String(id.description().into()),
            ])
        })
        .collect();
    Ok(Value::from_array(items))
}

fn security_use_provider_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let name = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("security_use_provider() expects provider name string".into()),
    };
    let id = ProviderId::from_name(name)
        .ok_or_else(|| format!("Unknown security provider: {}", name))?;
    get_security(env)?.with_security(|providers, _| {
        providers.set_active(id);
        Ok(Value::String(id.name().into()))
    })
}

fn security_provider_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let _ = args;
    let sec = get_security(env)?;
    let active = sec
        .providers
        .lock()
        .map_err(|_| "Security provider lock poisoned".to_string())?
        .active();
    Ok(Value::String(active.name().into()))
}

fn security_capabilities_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let _ = args;
    let sec = get_security(env)?;
    let active = sec
        .providers
        .lock()
        .map_err(|_| "Security provider lock poisoned".to_string())?
        .active();
    Ok(Value::from_array(
        active
            .capabilities()
            .iter()
            .map(|c| Value::String((*c).into()))
            .collect(),
    ))
}

fn device_list_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let _ = args;
    let sec = get_security(env)?;
    let devices = sec
        .devices
        .lock()
        .map_err(|_| "Device registry lock poisoned".to_string())?;
    Ok(Value::from_array(
        devices
            .list()
            .iter()
            .map(|d| {
                Value::from_array(vec![
                    Value::String(d.id.clone()),
                    Value::String(device_kind_label(d.kind).into()),
                    Value::String(d.name.clone()),
                ])
            })
            .collect(),
    ))
}

fn device_open_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("device_open() expects device id string".into()),
    };
    let handle = get_security(env)?.with_devices(|devices| devices.open(&id))?;
    Ok(Value::DeviceHandle(handle))
}

fn device_close_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::DeviceHandle(h)) => h.id,
        _ => return Err("device_close() expects device handle".into()),
    };
    get_security(env)?.with_devices(|devices| devices.close(handle))?;
    Ok(Value::Null)
}

fn device_read_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::DeviceHandle(h)) => h.id,
        _ => return Err("device_read() expects device handle".into()),
    };
    let len = args
        .get(1)
        .map(|v| expect_usize(v, 1, "device_read"))
        .transpose()?
        .unwrap_or(32);
    let sec = get_security(env)?;
    let devices = sec
        .devices
        .lock()
        .map_err(|_| "Device registry lock poisoned".to_string())?;
    Ok(bytes_to_array(&devices.read(handle, len)?))
}

fn device_write_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let handle = match args.first() {
        Some(Value::DeviceHandle(h)) => h.id,
        _ => return Err("device_write() expects device handle".into()),
    };
    let data = value_to_bytes(args.get(1).ok_or("device_write() expects data")?)?;
    let sec = get_security(env)?;
    let devices = sec
        .devices
        .lock()
        .map_err(|_| "Device registry lock poisoned".to_string())?;
    Ok(Value::Number(devices.write(handle, &data)? as i64))
}

pub fn security_globals(env: &mut Environment) {
    env.set(
        "security".to_string(),
        Value::SecurityHandle(SecurityHandle::new()),
    );

    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("crypto_random", crypto_random_native),
        ("crypto_sha3_256", crypto_sha3_256_native),
        ("crypto_sha3_512", crypto_sha3_512_native),
        ("crypto_argon2", crypto_argon2_native),
        ("crypto_aes256_encrypt", crypto_aes256_encrypt_native),
        ("crypto_aes256_decrypt", crypto_aes256_decrypt_native),
        ("crypto_chacha20_encrypt", crypto_chacha20_encrypt_native),
        ("crypto_chacha20_decrypt", crypto_chacha20_decrypt_native),
        ("crypto_rsa_generate", crypto_rsa_generate_native),
        ("crypto_rsa_encrypt", crypto_rsa_encrypt_native),
        ("crypto_rsa_decrypt", crypto_rsa_decrypt_native),
        ("crypto_ecc_generate", crypto_ecc_generate_native),
        ("crypto_ecc_sign", crypto_ecc_sign_native),
        ("crypto_ecc_verify", crypto_ecc_verify_native),
        ("crypto_kyber_encapsulate", crypto_kyber_encapsulate_native),
        ("crypto_dilithium_sign", crypto_dilithium_sign_native),
        ("crypto_secure", crypto_secure_native),
        ("crypto_wipe", crypto_wipe_native),
        ("crypto_is_secure", crypto_is_secure_native),
        ("security_list_providers", security_list_providers_native),
        ("security_use_provider", security_use_provider_native),
        ("security_provider", security_provider_native),
        ("security_capabilities", security_capabilities_native),
        ("device_list", device_list_native),
        ("device_open", device_open_native),
        ("device_close", device_close_native),
        ("device_read", device_read_native),
        ("device_write", device_write_native),
    ];

    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}

#[cfg(all(test, feature = "crypto"))]
mod integration_tests {
    use crate::evaluator::{create_global_env, eval_source};
    use crate::value::Value;

    #[test]
    fn eval_sha3_from_string() {
        let mut env = create_global_env();
        let val = eval_source("crypto_sha3_256(\"password\")", &mut env).expect("eval");
        match val {
            Value::Array(items) => assert_eq!(items.len(), 32),
            other => panic!("expected hash array, got {:?}", other),
        }
    }
}
