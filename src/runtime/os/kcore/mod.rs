//! Ring 0 kernel core — microkernel, executive, HAL, scheduler, dispatcher.

mod dispatcher;
mod executive;
mod hal;
mod microkernel;
mod sched;

pub use dispatcher::{ContextSwitch, Dispatcher};
pub use executive::Executive;
pub use hal::Hal;
pub use microkernel::{IpcMessage, Microkernel};
pub use sched::{FairScheduler, SchedPolicy, SchedTask};

use std::sync::atomic::{AtomicU64, Ordering};

/// Unified Ring-0 kernel core state.
pub struct KernelCore {
    pub microkernel: Microkernel,
    pub executive: Executive,
    pub hal: Hal,
    pub scheduler: FairScheduler,
    pub dispatcher: Dispatcher,
    pub ticks: AtomicU64,
}

impl Default for KernelCore {
    fn default() -> Self {
        Self {
            microkernel: Microkernel::default(),
            executive: Executive::default(),
            hal: Hal::default(),
            scheduler: FairScheduler::default(),
            dispatcher: Dispatcher::default(),
            ticks: AtomicU64::new(0),
        }
    }
}

impl KernelCore {
    pub fn tick(&mut self) -> Option<SchedTask> {
        self.ticks.fetch_add(1, Ordering::SeqCst);
        self.hal.advance_timer();
        self.scheduler.tick()
    }

    pub fn ipc_send(&mut self, from: u64, to: u64, payload: Vec<u8>) -> Result<(), String> {
        self.microkernel.send(from, to, payload)
    }

    pub fn ipc_recv(&mut self, endpoint: u64) -> Option<IpcMessage> {
        self.microkernel.recv(endpoint)
    }

    pub fn context_switch(&mut self, from_tid: u64, to_tid: u64) -> ContextSwitch {
        self.dispatcher.switch(from_tid, to_tid)
    }

    pub fn info(&self) -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("ring".into(), "0".into());
        m.insert("arch".into(), self.hal.arch.as_str().into());
        m.insert(
            "scheduler".into(),
            match self.scheduler.policy {
                SchedPolicy::Cfs => "cfs",
                SchedPolicy::Realtime => "rt",
            }
            .into(),
        );
        m.insert(
            "ipc_endpoints".into(),
            self.microkernel.endpoint_count().to_string(),
        );
        m.insert(
            "context_switches".into(),
            self.dispatcher.switch_count().to_string(),
        );
        m.insert("ticks".into(), self.ticks.load(Ordering::SeqCst).to_string());
        m
    }
}
