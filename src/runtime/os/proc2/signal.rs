//! Signal dispatcher — SIGKILL, SIGTERM, user signals.

use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Kill = 9,
    Term = 15,
    User1 = 10,
    User2 = 12,
    Hup = 1,
}

impl Signal {
    pub fn from_num(n: i32) -> Option<Self> {
        match n {
            9 => Some(Signal::Kill),
            15 => Some(Signal::Term),
            10 => Some(Signal::User1),
            12 => Some(Signal::User2),
            1 => Some(Signal::Hup),
            _ => None,
        }
    }

    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

pub struct SignalHandler {
    pending: HashMap<u64, VecDeque<Signal>>,
    handlers: HashMap<u64, HashMap<i32, String>>,
}

impl Default for SignalHandler {
    fn default() -> Self {
        Self {
            pending: HashMap::new(),
            handlers: HashMap::new(),
        }
    }
}

impl SignalHandler {
    pub fn deliver(&mut self, pid: u64, sig: Signal) -> bool {
        self.pending.entry(pid).or_default().push_back(sig);
        true
    }

    pub fn register(&mut self, pid: u64, sig: Signal, handler: &str) {
        self.handlers
            .entry(pid)
            .or_default()
            .insert(sig.as_i32(), handler.to_string());
    }

    pub fn pending(&mut self, pid: u64) -> Option<Signal> {
        self.pending.get_mut(&pid)?.pop_front()
    }
}
