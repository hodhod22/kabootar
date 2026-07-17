//! Network stack — Ethernet/IP/TCP/UDP layers + traffic control.

mod layers;
mod traffic;

pub use layers::NetStack;
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

    pub fn info(&self) -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("packets".into(), self.stack.packet_count().to_string());
        m.insert("layers".into(), self.stack.layers().len().to_string());
        m.insert("qos".into(), "enabled".into());
        m
    }
}
