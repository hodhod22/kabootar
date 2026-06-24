//! Error handling — crash reports and watchdog timer.

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct CrashReport {
    pub code: String,
    pub message: String,
    pub timestamp: u64,
}

pub struct Watchdog {
    timeout_ms: u64,
    last_ping: u64,
    tripped: bool,
}

impl Default for Watchdog {
    fn default() -> Self {
        Self {
            timeout_ms: 30_000,
            last_ping: now_ms(),
            tripped: false,
        }
    }
}

impl Watchdog {
    pub fn ping(&mut self) {
        self.last_ping = now_ms();
        self.tripped = false;
    }

    pub fn check(&mut self) -> bool {
        if now_ms().saturating_sub(self.last_ping) > self.timeout_ms {
            self.tripped = true;
        }
        self.tripped
    }
}

pub struct ErrorSubsystem {
    pub watchdog: Watchdog,
    crashes: Vec<CrashReport>,
}

impl Default for ErrorSubsystem {
    fn default() -> Self {
        Self {
            watchdog: Watchdog::default(),
            crashes: Vec::new(),
        }
    }
}

impl ErrorSubsystem {
    pub fn panic(&mut self, code: &str, message: &str) -> CrashReport {
        let report = CrashReport {
            code: code.to_string(),
            message: message.to_string(),
            timestamp: now_ms(),
        };
        self.crashes.push(report.clone());
        report
    }

    pub fn crash_count(&self) -> usize {
        self.crashes.len()
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
