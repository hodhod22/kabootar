//! Thread pool — lightweight execution threads per process.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone)]
pub struct Thread {
    pub tid: u64,
    pub pid: u64,
    pub name: String,
    pub state: String,
}

pub struct ThreadPool {
    next_tid: AtomicU64,
    threads: HashMap<u64, Thread>,
}

impl Default for ThreadPool {
    fn default() -> Self {
        Self {
            next_tid: AtomicU64::new(1),
            threads: HashMap::new(),
        }
    }
}

impl ThreadPool {
    pub fn spawn_main(&mut self, pid: u64) -> u64 {
        self.spawn(pid, "main")
    }

    pub fn spawn(&mut self, pid: u64, name: &str) -> u64 {
        let tid = self.next_tid.fetch_add(1, Ordering::SeqCst);
        self.threads.insert(
            tid,
            Thread {
                tid,
                pid,
                name: name.to_string(),
                state: "running".into(),
            },
        );
        tid
    }

    pub fn list(&self) -> Vec<Thread> {
        let mut v: Vec<_> = self.threads.values().cloned().collect();
        v.sort_by_key(|t| t.tid);
        v
    }

    pub fn count(&self) -> usize {
        self.threads.len()
    }
}
