//! Event logging — ring buffers and kernel telemetry.

use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: String,
    pub message: String,
    pub timestamp: u64,
}

pub struct EventLog {
    ring: VecDeque<LogEntry>,
    capacity: usize,
    total: u64,
}

impl Default for EventLog {
    fn default() -> Self {
        Self {
            ring: VecDeque::new(),
            capacity: 1024,
            total: 0,
        }
    }
}

impl EventLog {
    pub fn record(&mut self, level: &str, message: &str) {
        self.total += 1;
        if self.ring.len() >= self.capacity {
            self.ring.pop_front();
        }
        self.ring.push_back(LogEntry {
            level: level.to_string(),
            message: message.to_string(),
            timestamp: self.total,
        });
    }

    pub fn drain(&mut self, max: usize) -> Vec<LogEntry> {
        let n = max.min(self.ring.len());
        self.ring.drain(..n).collect()
    }

    pub fn total(&self) -> u64 {
        self.total
    }
}
