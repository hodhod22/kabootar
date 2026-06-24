use super::devices::{DeviceKind, DeviceRegistry};
#[cfg(feature = "crypto")]
use rand::RngCore;

/// Pluggable security backend — Kabootar ships stubs; apps choose the war, not Kabootar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderId {
    Software,
    TpmStub,
    YubiKeyStub,
    HsmStub,
}

impl ProviderId {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "software" => Some(Self::Software),
            "tpm" | "tpm-stub" => Some(Self::TpmStub),
            "yubikey" | "yubikey-stub" => Some(Self::YubiKeyStub),
            "hsm" | "hsm-stub" => Some(Self::HsmStub),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Software => "software",
            Self::TpmStub => "tpm-stub",
            Self::YubiKeyStub => "yubikey-stub",
            Self::HsmStub => "hsm-stub",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Software => "CPU software crypto (default toolbox)",
            Self::TpmStub => "Trusted Platform Module stub — swap for real TPM driver",
            Self::YubiKeyStub => "Smart card / YubiKey-class stub",
            Self::HsmStub => "Hardware Security Module stub (quorum/HSM workflows)",
        }
    }

    pub fn capabilities(self) -> &'static [&'static str] {
        match self {
            Self::Software => &[
                "aes-256-gcm",
                "chacha20-poly1305",
                "sha3",
                "argon2",
                "rsa",
                "ecc-p256",
                "random",
            ],
            Self::TpmStub => &["random", "seal", "attest"],
            Self::YubiKeyStub => &["random", "sign", "otp"],
            Self::HsmStub => &["random", "sign", "quorum"],
        }
    }

    pub fn all() -> &'static [ProviderId] {
        &[
            ProviderId::Software,
            ProviderId::TpmStub,
            ProviderId::YubiKeyStub,
            ProviderId::HsmStub,
        ]
    }
}

pub struct ProviderRegistry {
    active: ProviderId,
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self {
            active: ProviderId::Software,
        }
    }
}

impl ProviderRegistry {
    pub fn active(&self) -> ProviderId {
        self.active
    }

    pub fn set_active(&mut self, id: ProviderId) {
        self.active = id;
    }

    pub fn random_bytes(&self, devices: &mut DeviceRegistry, len: usize) -> Result<Vec<u8>, String> {
        let len = len.clamp(1, 65536);
        let mut buf = vec![0u8; len];
        match self.active {
            ProviderId::Software => {
                #[cfg(feature = "crypto")]
                {
                    rand::thread_rng().fill_bytes(&mut buf);
                }
                #[cfg(not(feature = "crypto"))]
                {
                    return Err(
                        "Crypto not enabled. Rebuild with: cargo build --features crypto".into(),
                    );
                }
            }
            ProviderId::TpmStub => {
                let handle = devices
                    .open("tpm-0")
                    .map_err(|e| format!("TPM provider: {}", e))?;
                buf = devices.read(handle.id, len)?;
                devices.close(handle.id)?;
            }
            ProviderId::YubiKeyStub => {
                let handle = devices
                    .open("sc-0")
                    .map_err(|e| format!("YubiKey provider: {}", e))?;
                buf = devices.read(handle.id, len)?;
                devices.close(handle.id)?;
            }
            ProviderId::HsmStub => {
                let handle = devices
                    .open("usb-0")
                    .map_err(|e| format!("HSM provider: {}", e))?;
                buf = devices.read(handle.id, len)?;
                devices.close(handle.id)?;
            }
        }
        Ok(buf)
    }
}

pub fn device_kind_label(kind: DeviceKind) -> &'static str {
    kind.as_str()
}
