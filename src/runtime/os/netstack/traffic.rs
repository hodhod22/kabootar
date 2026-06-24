//! Traffic control — QoS packet prioritization.

pub struct TrafficControl {
    voip_priority: u8,
    bulk_priority: u8,
}

impl Default for TrafficControl {
    fn default() -> Self {
        Self {
            voip_priority: 7,
            bulk_priority: 1,
        }
    }
}

impl TrafficControl {
    pub fn classify(&self, proto: &str) -> u8 {
        match proto.to_ascii_lowercase().as_str() {
            "voip" | "rtp" | "sip" => self.voip_priority,
            "tcp" => 4,
            "udp" => 3,
            _ => self.bulk_priority,
        }
    }

    pub fn set_priority(&mut self, voip: u8, bulk: u8) {
        self.voip_priority = voip;
        self.bulk_priority = bulk;
    }
}
