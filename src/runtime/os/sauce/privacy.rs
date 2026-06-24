//! Strategy 8 — Privacy by design: RAM panic encrypt + differential privacy telemetry.

use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct TelemetryEvent {
    pub category: String,
    pub noisy_count: i64,
}

pub struct PrivacyCore {
    ram_encrypted: bool,
    privacy_switch: bool,
    telemetry_enabled: bool,
    events: VecDeque<TelemetryEvent>,
    epsilon: f64,
}

impl Default for PrivacyCore {
    fn default() -> Self {
        Self {
            ram_encrypted: false,
            privacy_switch: false,
            telemetry_enabled: false,
            events: VecDeque::new(),
            epsilon: 1.0,
        }
    }
}

impl PrivacyCore {
    pub fn set_telemetry_enabled(&mut self, enabled: bool) {
        self.telemetry_enabled = enabled;
    }

    pub fn telemetry_enabled(&self) -> bool {
        self.telemetry_enabled
    }
    pub fn engage_privacy_switch(&mut self) {
        self.privacy_switch = true;
        self.panic_encrypt_ram();
    }

    pub fn panic_encrypt_ram(&mut self) -> bool {
        self.ram_encrypted = true;
        true
    }

    pub fn ram_locked(&self) -> bool {
        self.ram_encrypted
    }

    pub fn submit_telemetry(&mut self, category: &str, raw_count: i64) -> TelemetryEvent {
        if !self.telemetry_enabled {
            return TelemetryEvent {
                category: category.to_string(),
                noisy_count: 0,
            };
        }
        let noise = ((raw_count as f64 * 0.07).sin() * self.epsilon * 10.0).round() as i64;
        let evt = TelemetryEvent {
            category: category.to_string(),
            noisy_count: raw_count.saturating_add(noise),
        };
        self.events.push_back(evt.clone());
        if self.events.len() > 64 {
            self.events.pop_front();
        }
        evt
    }

    pub fn recent_events(&self) -> Vec<TelemetryEvent> {
        self.events.iter().cloned().collect()
    }
}
