//! Strategy 5 — Energy arbitrage: schedule background work on wall power only.

use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct BackgroundJob {
    pub name: String,
    pub wall_power_only: bool,
    pub paused: bool,
}

pub struct EnergyCore {
    on_battery: bool,
    active_app: String,
    queue: VecDeque<BackgroundJob>,
    deferred: u64,
    eco_mode: bool,
    last_repaint_ms: u64,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl Default for EnergyCore {
    fn default() -> Self {
        Self {
            on_battery: true,
            active_app: "desktop".into(),
            queue: VecDeque::new(),
            deferred: 0,
            eco_mode: false,
            last_repaint_ms: 0,
        }
    }
}

impl EnergyCore {
    pub fn set_power_source(&mut self, on_battery: bool) {
        self.on_battery = on_battery;
        if !on_battery {
            self.flush_deferred();
        }
    }

    pub fn set_eco_mode(&mut self, on: bool) {
        self.eco_mode = on;
    }

    pub fn set_active_app(&mut self, app: &str) {
        self.active_app = app.to_string();
    }

    pub fn schedule(&mut self, name: &str, wall_power_only: bool) -> bool {
        let job = BackgroundJob {
            name: name.to_string(),
            wall_power_only,
            paused: false,
        };
        if self.on_battery && wall_power_only {
            self.deferred += 1;
            self.queue.push_back(job);
            false
        } else {
            self.queue.push_back(job);
            true
        }
    }

    pub fn should_repaint(&self) -> bool {
        if !self.on_battery && !self.eco_mode {
            return true;
        }
        let min_interval = if self.on_battery { 1000 } else { 33 };
        now_ms().saturating_sub(self.last_repaint_ms) >= min_interval
    }

    pub fn mark_repaint(&mut self) {
        self.last_repaint_ms = now_ms();
    }

    fn flush_deferred(&mut self) {
        for job in self.queue.iter_mut() {
            if job.wall_power_only {
                job.paused = false;
            }
        }
        self.deferred = 0;
    }

    pub fn stats(&self) -> (bool, String, u64, usize) {
        (
            self.on_battery,
            self.active_app.clone(),
            self.deferred,
            self.queue.len(),
        )
    }
}
