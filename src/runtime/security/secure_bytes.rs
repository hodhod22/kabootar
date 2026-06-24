use std::sync::{Arc, Mutex};

/// Shared sensitive buffer — `crypto_wipe()` zeroes all clones via Arc.
#[derive(Clone)]
pub struct SecureBytes(Arc<Mutex<Vec<u8>>>);

impl std::fmt::Debug for SecureBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<secure bytes>")
    }
}

impl SecureBytes {
    pub fn from_vec(data: Vec<u8>) -> Self {
        Self(Arc::new(Mutex::new(data)))
    }

    pub fn read(&self) -> Result<Vec<u8>, String> {
        let guard = self
            .0
            .lock()
            .map_err(|_| "Secure buffer lock poisoned".to_string())?;
        Ok(guard.clone())
    }

    pub fn len(&self) -> Result<usize, String> {
        let guard = self
            .0
            .lock()
            .map_err(|_| "Secure buffer lock poisoned".to_string())?;
        Ok(guard.len())
    }

    pub fn wipe(&self) -> Result<(), String> {
        let mut guard = self
            .0
            .lock()
            .map_err(|_| "Secure buffer lock poisoned".to_string())?;
        for byte in guard.iter_mut() {
            *byte = 0;
        }
        guard.clear();
        #[cfg(feature = "crypto")]
        {
            // Extra assurance when zeroize crate is linked.
            use zeroize::Zeroize;
            guard.zeroize();
        }
        Ok(())
    }
}
