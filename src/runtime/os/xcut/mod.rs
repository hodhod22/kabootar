//! Cross-cutting systems — security monitor, error handling, logging, power.

mod error;
mod log;
mod power;
mod security;

pub use error::ErrorSubsystem;
pub use log::EventLog;
pub use power::PowerManager;
pub use security::SecurityMonitor;

pub struct CrosscutSubsystem {
    pub security: SecurityMonitor,
    pub error: ErrorSubsystem,
    pub log: EventLog,
    pub power: PowerManager,
}

impl Default for CrosscutSubsystem {
    fn default() -> Self {
        Self {
            security: SecurityMonitor::default(),
            error: ErrorSubsystem::default(),
            log: EventLog::default(),
            power: PowerManager::default(),
        }
    }
}

impl CrosscutSubsystem {
    pub fn audit(&mut self, pid: u64, action: &str, allowed: bool) {
        self.security.audit(pid, action, allowed);
        self.log.record(
            if allowed { "AUDIT_OK" } else { "AUDIT_DENY" },
            &format!("pid={pid} {action}"),
        );
    }
}
