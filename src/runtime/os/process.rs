//! Process table for Kabootar OS (layer 2) — modeled after host OS process APIs.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone)]
pub struct ProcessEntry {
    pub pid: u64,
    pub name: String,
    pub state: ProcessState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessState {
    Running,
    Stopped,
}

pub struct ProcessTable {
    next_pid: AtomicU64,
    processes: HashMap<u64, ProcessEntry>,
}

impl Default for ProcessTable {
    fn default() -> Self {
        let mut processes = HashMap::new();
        processes.insert(
            1,
            ProcessEntry {
                pid: 1,
                name: "kabootar-init".into(),
                state: ProcessState::Running,
            },
        );
        Self {
            next_pid: AtomicU64::new(2),
            processes,
        }
    }
}

impl ProcessTable {
    pub fn spawn(&mut self, name: &str) -> u64 {
        let pid = self.next_pid.fetch_add(1, Ordering::SeqCst);
        self.processes.insert(
            pid,
            ProcessEntry {
                pid,
                name: name.to_string(),
                state: ProcessState::Running,
            },
        );
        pid
    }

    pub fn list(&self) -> Vec<ProcessEntry> {
        let mut out: Vec<_> = self.processes.values().cloned().collect();
        out.sort_by_key(|p| p.pid);
        out
    }

    pub fn kill(&mut self, pid: u64) -> bool {
        if pid == 1 {
            return false;
        }
        if let Some(p) = self.processes.get_mut(&pid) {
            p.state = ProcessState::Stopped;
            true
        } else {
            false
        }
    }
}
