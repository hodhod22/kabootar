//! Dispatcher — context switch between threads.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone)]
pub struct ThreadContext {
    pub tid: u64,
    pub pid: u64,
    pub stack_ptr: u64,
    pub instruction_ptr: u64,
}

#[derive(Debug, Clone)]
pub struct ContextSwitch {
    pub from: u64,
    pub to: u64,
    pub saved_sp: u64,
    pub saved_ip: u64,
    pub elapsed_ns: u64,
}

pub struct Dispatcher {
    contexts: HashMap<u64, ThreadContext>,
    switches: AtomicU64,
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self {
            contexts: HashMap::new(),
            switches: AtomicU64::new(0),
        }
    }
}

impl Dispatcher {
    pub fn register_thread(&mut self, tid: u64, pid: u64) {
        self.contexts.insert(
            tid,
            ThreadContext {
                tid,
                pid,
                stack_ptr: 0x7fff_0000 + tid * 0x10000,
                instruction_ptr: 0,
            },
        );
    }

    pub fn switch(&mut self, from_tid: u64, to_tid: u64) -> ContextSwitch {
        let start = std::time::Instant::now();
        self.switches.fetch_add(1, Ordering::SeqCst);
        let to = self
            .contexts
            .entry(to_tid)
            .or_insert(ThreadContext {
                tid: to_tid,
                pid: 0,
                stack_ptr: 0x7fff_0000,
                instruction_ptr: 0,
            });
        let elapsed_ns = start.elapsed().as_nanos() as u64;
        ContextSwitch {
            from: from_tid,
            to: to_tid,
            saved_sp: to.stack_ptr,
            saved_ip: to.instruction_ptr,
            elapsed_ns,
        }
    }

    pub fn switch_count(&self) -> u64 {
        self.switches.load(Ordering::SeqCst)
    }
}
