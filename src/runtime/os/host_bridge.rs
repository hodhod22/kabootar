//! Host bridge — forward OS driver I/O to the native host (files, serial path).

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use std::io::{Read, Write};
    use std::path::PathBuf;

    #[derive(Debug, Clone, Default)]
    pub struct HostBridge {
        audio_pcm_path: Option<PathBuf>,
        usb_serial_path: Option<PathBuf>,
        enabled: bool,
    }

    impl HostBridge {
        pub fn from_env() -> Self {
            let bridge_on = std::env::var("KABOOTAR_HOST_BRIDGE")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false);
            let audio_pcm_path = std::env::var("KABOOTAR_HOST_AUDIO")
                .ok()
                .map(PathBuf::from)
                .or_else(|| if bridge_on { default_audio_path() } else { None });
            let usb_serial_path = std::env::var("KABOOTAR_HOST_USB").ok().map(PathBuf::from);
            let enabled = bridge_on || audio_pcm_path.is_some() || usb_serial_path.is_some();
            Self {
                audio_pcm_path,
                usb_serial_path,
                enabled,
            }
        }

        pub fn is_enabled(&self) -> bool {
            self.enabled
        }

        pub fn info(&self) -> std::collections::HashMap<String, String> {
            let mut m = std::collections::HashMap::new();
            m.insert("enabled".into(), self.enabled.to_string());
            m.insert(
                "audio".into(),
                self.audio_pcm_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "none".into()),
            );
            m.insert(
                "usb".into(),
                self.usb_serial_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "none".into()),
            );
            m
        }

        pub fn audio_write(&self, samples: &[i16]) -> Result<usize, String> {
            if !self.enabled {
                return Ok(0);
            }
            let path = self
                .audio_pcm_path
                .as_ref()
                .ok_or("host audio path not configured (KABOOTAR_HOST_AUDIO)")?;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|e| format!("host audio open: {e}"))?;
            let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
            file.write_all(&bytes)
                .map_err(|e| format!("host audio write: {e}"))?;
            Ok(samples.len())
        }

        pub fn usb_serial_write(&self, data: &[u8]) -> Result<usize, String> {
            if !self.enabled {
                return Ok(0);
            }
            let path = self
                .usb_serial_path
                .as_ref()
                .ok_or("host usb path not configured (KABOOTAR_HOST_USB)")?;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|e| format!("host usb open: {e}"))?;
            file.write_all(data)
                .map_err(|e| format!("host usb write: {e}"))?;
            Ok(data.len())
        }

        pub fn usb_serial_read(&self, max: usize) -> Result<Vec<u8>, String> {
            if !self.enabled {
                return Ok(Vec::new());
            }
            let path = self
                .usb_serial_path
                .as_ref()
                .ok_or("host usb path not configured (KABOOTAR_HOST_USB)")?;
            if !path.exists() {
                return Ok(Vec::new());
            }
            let mut file = std::fs::File::open(path).map_err(|e| format!("host usb read: {e}"))?;
            let mut buf = vec![0u8; max.min(4096)];
            let n = file.read(&mut buf).map_err(|e| format!("host usb read: {e}"))?;
            buf.truncate(n);
            Ok(buf)
        }
    }

    fn default_audio_path() -> Option<PathBuf> {
        Some(std::env::temp_dir().join("kabootar-host-audio.pcm"))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use imp::HostBridge;

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Default)]
pub struct HostBridge;

#[cfg(target_arch = "wasm32")]
impl HostBridge {
    pub fn from_env() -> Self {
        Self
    }

    pub fn is_enabled(&self) -> bool {
        false
    }

    pub fn info(&self) -> std::collections::HashMap<String, String> {
        std::collections::HashMap::from([
            ("enabled".into(), "false".into()),
            ("audio".into(), "none".into()),
            ("usb".into(), "none".into()),
        ])
    }

    pub fn audio_write(&self, _samples: &[i16]) -> Result<usize, String> {
        Ok(0)
    }

    pub fn usb_serial_write(&self, _data: &[u8]) -> Result<usize, String> {
        Ok(0)
    }

    pub fn usb_serial_read(&self, _max: usize) -> Result<Vec<u8>, String> {
        Ok(Vec::new())
    }
}
