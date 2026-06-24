//! Strategy 6 — Emotional UI: spring physics + pre-emptive danger haptics.

#[derive(Debug, Clone)]
pub struct HapticEvent {
    pub action: String,
    pub stiffness: f32,
    pub damping: f32,
}

#[derive(Debug, Clone)]
pub struct DangerFeedback {
    pub path: String,
    pub glow: String,
    pub vibrate: u8,
    pub blocked: bool,
}

pub struct HapticUi {
    stiffness: f32,
    events: u64,
}

impl Default for HapticUi {
    fn default() -> Self {
        Self {
            stiffness: 420.0,
            events: 0,
        }
    }
}

impl HapticUi {
    pub fn spring_for(&self, action: &str) -> HapticEvent {
        HapticEvent {
            action: action.to_string(),
            stiffness: self.stiffness,
            damping: 28.0,
        }
    }

    pub fn set_stiffness(&mut self, k: f32) {
        self.stiffness = k.max(1.0);
    }

    pub fn danger_feedback(&mut self, path: &str, important: bool) -> DangerFeedback {
        self.events += 1;
        DangerFeedback {
            path: path.to_string(),
            glow: if important { "red-pulse" } else { "amber" }.into(),
            vibrate: if important { 3 } else { 1 },
            blocked: important && path.starts_with("/system"),
        }
    }

    pub fn event_count(&self) -> u64 {
        self.events
    }
}
