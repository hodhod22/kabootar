//! Microkernel — IPC and address-space bookkeeping (Ring 0 minimum).

use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone)]
pub struct IpcMessage {
    pub from: u64,
    pub to: u64,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct IpcEndpoint {
    pub id: u64,
    pub owner_pid: u64,
    pub name: String,
}

pub struct Microkernel {
    next_ep: u64,
    endpoints: HashMap<u64, IpcEndpoint>,
    mailboxes: HashMap<u64, VecDeque<IpcMessage>>,
    address_spaces: HashMap<u64, u64>,
}

impl Default for Microkernel {
    fn default() -> Self {
        let mut mk = Self {
            next_ep: 1,
            endpoints: HashMap::new(),
            mailboxes: HashMap::new(),
            address_spaces: HashMap::new(),
        };
        mk.register_endpoint(1, "init").ok();
        mk
    }
}

impl Microkernel {
    pub fn register_endpoint(&mut self, owner_pid: u64, name: &str) -> Result<u64, String> {
        let id = self.next_ep;
        self.next_ep += 1;
        self.endpoints.insert(
            id,
            IpcEndpoint {
                id,
                owner_pid,
                name: name.to_string(),
            },
        );
        self.mailboxes.insert(id, VecDeque::new());
        Ok(id)
    }

    pub fn map_address_space(&mut self, pid: u64, pages: u64) {
        self.address_spaces.insert(pid, pages);
    }

    pub fn send(&mut self, from: u64, to: u64, payload: Vec<u8>) -> Result<(), String> {
        if !self.endpoints.contains_key(&to) {
            return Err(format!("ipc endpoint not found: {to}"));
        }
        self.mailboxes
            .get_mut(&to)
            .ok_or_else(|| "ipc mailbox missing".to_string())?
            .push_back(IpcMessage { from, to, payload });
        Ok(())
    }

    pub fn recv(&mut self, endpoint: u64) -> Option<IpcMessage> {
        self.mailboxes.get_mut(&endpoint)?.pop_front()
    }

    pub fn endpoint_count(&self) -> usize {
        self.endpoints.len()
    }

    pub fn list_endpoints(&self) -> Vec<IpcEndpoint> {
        let mut v: Vec<_> = self.endpoints.values().cloned().collect();
        v.sort_by_key(|e| e.id);
        v
    }
}
