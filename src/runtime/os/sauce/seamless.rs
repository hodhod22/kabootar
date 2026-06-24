//! Strategy 4 — Seamless ecosystem: proximity unlock + ultrasonic pairing + clipboard.

use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct PairedDevice {
    pub id: String,
    pub kind: String,
    pub ultrasonic_hz: u32,
}

pub struct SeamlessEcosystem {
    paired: Vec<PairedDevice>,
    clipboard: VecDeque<String>,
    unlock_device: Option<String>,
}

impl Default for SeamlessEcosystem {
    fn default() -> Self {
        Self {
            paired: Vec::new(),
            clipboard: VecDeque::new(),
            unlock_device: None,
        }
    }
}

impl SeamlessEcosystem {
    pub fn pair_ultrasonic(&mut self, freq_hz: u32) -> String {
        let id = format!("ultra-{freq_hz}");
        self.paired.push(PairedDevice {
            id: id.clone(),
            kind: if freq_hz > 18_000 { "phone" } else { "unknown" }.into(),
            ultrasonic_hz: freq_hz,
        });
        id
    }

    pub fn unlock_proximity(&mut self, device_id: &str) -> bool {
        if self.paired.iter().any(|d| d.id == device_id) {
            self.unlock_device = Some(device_id.to_string());
            true
        } else {
            false
        }
    }

    pub fn clipboard_push(&mut self, text: &str) {
        self.clipboard.push_back(text.to_string());
        if self.clipboard.len() > 32 {
            self.clipboard.pop_front();
        }
    }

    pub fn clipboard_poll(&mut self) -> Option<String> {
        self.clipboard.pop_front()
    }

    pub fn paired_count(&self) -> usize {
        self.paired.len()
    }

    pub fn list_paired(&self) -> Vec<PairedDevice> {
        self.paired.clone()
    }

    pub fn unlocked_by(&self) -> Option<&str> {
        self.unlock_device.as_deref()
    }
}
