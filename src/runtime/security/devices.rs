use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Usb,
    Tpm,
    SmartCard,
}

impl DeviceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            DeviceKind::Usb => "usb",
            DeviceKind::Tpm => "tpm",
            DeviceKind::SmartCard => "smartcard",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub id: String,
    pub kind: DeviceKind,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct DeviceHandle {
    pub id: u64,
    pub device_id: String,
    pub kind: DeviceKind,
}

static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

pub struct DeviceRegistry {
    devices: Vec<DeviceInfo>,
    open: HashMap<u64, DeviceInfo>,
}

impl Default for DeviceRegistry {
    fn default() -> Self {
        Self {
            devices: vec![
                DeviceInfo {
                    id: "usb-0".into(),
                    kind: DeviceKind::Usb,
                    name: "Generic USB token (stub)".into(),
                },
                DeviceInfo {
                    id: "tpm-0".into(),
                    kind: DeviceKind::Tpm,
                    name: "Platform TPM 2.0 (stub)".into(),
                },
                DeviceInfo {
                    id: "sc-0".into(),
                    kind: DeviceKind::SmartCard,
                    name: "Smart card / YubiKey-class (stub)".into(),
                },
            ],
            open: HashMap::new(),
        }
    }
}

impl DeviceRegistry {
    pub fn list(&self) -> &[DeviceInfo] {
        &self.devices
    }

    pub fn open(&mut self, device_id: &str) -> Result<DeviceHandle, String> {
        let info = self
            .devices
            .iter()
            .find(|d| d.id == device_id)
            .ok_or_else(|| format!("Device not found: {}", device_id))?
            .clone();
        let id = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
        self.open.insert(id, info.clone());
        Ok(DeviceHandle {
            id,
            device_id: info.id,
            kind: info.kind,
        })
    }

    pub fn close(&mut self, handle_id: u64) -> Result<(), String> {
        self.open
            .remove(&handle_id)
            .map(|_| ())
            .ok_or_else(|| format!("Invalid device handle: {}", handle_id))
    }

    pub fn read(&self, handle_id: u64, len: usize) -> Result<Vec<u8>, String> {
        let info = self
            .open
            .get(&handle_id)
            .ok_or_else(|| format!("Invalid device handle: {}", handle_id))?;
        let len = len.clamp(1, 4096);
        Ok(match info.kind {
            DeviceKind::Tpm => vec![0xAA; len],
            DeviceKind::SmartCard => vec![0x55; len],
            DeviceKind::Usb => vec![0x01; len],
        })
    }

    pub fn write(&self, handle_id: u64, data: &[u8]) -> Result<usize, String> {
        let info = self
            .open
            .get(&handle_id)
            .ok_or_else(|| format!("Invalid device handle: {}", handle_id))?;
        if data.is_empty() {
            return Err("device_write() expects non-empty data".into());
        }
        Ok(match info.kind {
            DeviceKind::Tpm | DeviceKind::SmartCard | DeviceKind::Usb => data.len(),
        })
    }
}
