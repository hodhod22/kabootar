//! Strategy 2 — "Tears of Joy" install: 90s USB-to-desktop, zero-touch NFC setup.

use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct DeviceProfile {
    pub wifi_ssid: String,
    pub language: String,
    pub timezone: String,
    pub dark_theme: bool,
    pub diagnostics_opt_in: bool,
}

pub struct ZeroTouchSetup {
    started: Instant,
    target_secs: u64,
    profile: Option<DeviceProfile>,
    complete: bool,
}

impl Default for ZeroTouchSetup {
    fn default() -> Self {
        Self {
            started: Instant::now(),
            target_secs: 90,
            profile: None,
            complete: false,
        }
    }
}

impl ZeroTouchSetup {
    pub fn elapsed_secs(&self) -> u64 {
        self.started.elapsed().as_secs()
    }

    pub fn target_secs(&self) -> u64 {
        self.target_secs
    }

    pub fn nfc_bump(&mut self, token: &str) -> Result<DeviceProfile, String> {
        if token.is_empty() {
            return Err("nfc token required".into());
        }
        let profile = DeviceProfile {
            wifi_ssid: format!("cloned-from-{token}"),
            language: "sv-SE".into(),
            timezone: "Europe/Stockholm".into(),
            dark_theme: true,
            diagnostics_opt_in: false,
        };
        self.profile = Some(profile.clone());
        if self.elapsed_secs() <= self.target_secs {
            self.complete = true;
        }
        Ok(profile)
    }

    pub fn is_complete(&self) -> bool {
        self.complete
    }

    pub fn profile(&self) -> Option<&DeviceProfile> {
        self.profile.as_ref()
    }

    pub fn install_budget(&self) -> Duration {
        Duration::from_secs(self.target_secs)
    }
}
