//! Protocol stack layers — Ethernet → IP → TCP/UDP.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolLayer {
    Ethernet,
    Arp,
    Ip,
    Icmp,
    Tcp,
    Udp,
    Socket,
}

pub struct NetStack {
    layers: Vec<ProtocolLayer>,
    packets: u64,
}

impl Default for NetStack {
    fn default() -> Self {
        Self {
            layers: vec![
                ProtocolLayer::Ethernet,
                ProtocolLayer::Ip,
                ProtocolLayer::Tcp,
                ProtocolLayer::Udp,
                ProtocolLayer::Socket,
            ],
            packets: 0,
        }
    }
}

impl NetStack {
    pub fn layers(&self) -> &[ProtocolLayer] {
        &self.layers
    }

    pub fn transmit(&mut self, proto: &str, payload: &[u8], qos_class: u8) -> Result<Vec<u8>, String> {
        self.packets += 1;
        let header = match proto.to_ascii_lowercase().as_str() {
            "tcp" => format!("TCP qos={qos_class} len={}", payload.len()),
            "udp" => format!("UDP qos={qos_class} len={}", payload.len()),
            "icmp" => format!("ICMP len={}", payload.len()),
            _ => format!("IP proto={proto} len={}", payload.len()),
        };
        let mut out = header.into_bytes();
        out.extend_from_slice(payload);
        Ok(out)
    }

    pub fn packet_count(&self) -> u64 {
        self.packets
    }
}
