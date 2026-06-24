//! Network stack — Ethernet/IP/TCP/UDP layers + traffic control.

mod layers;
mod traffic;

pub use layers::{NetStack, ProtocolLayer};
pub use traffic::TrafficControl;

pub struct NetStackSubsystem {
    pub stack: NetStack,
    pub traffic: TrafficControl,
}

impl Default for NetStackSubsystem {
    fn default() -> Self {
        Self {
            stack: NetStack::default(),
            traffic: TrafficControl::default(),
        }
    }
}

impl NetStackSubsystem {
    pub fn send_packet(&mut self, proto: &str, payload: &[u8]) -> Result<Vec<u8>, String> {
        let class = self.traffic.classify(proto);
        self.stack.transmit(proto, payload, class)
    }
}
