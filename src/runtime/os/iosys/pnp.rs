//! Plug-and-Play — dynamic device discovery.

use std::collections::HashMap;

pub struct PnpManager {
    devices: HashMap<String, String>,
}

impl Default for PnpManager {
    fn default() -> Self {
        let mut d = HashMap::new();
        d.insert("usb:046d:c52b".into(), "hid-generic".into());
        d.insert("pci:8086:15f3".into(), "net-e1000".into());
        d.insert("usb:0781:5567".into(), "mass-storage".into());
        Self { devices: d }
    }
}

impl PnpManager {
    pub fn discover(&self, bus: &str, vid: &str, pid: &str) -> Option<String> {
        let key = format!("{bus}:{vid}:{pid}");
        self.devices.get(&key).cloned()
    }

    pub fn add(&mut self, bus: &str, vid: &str, pid: &str, driver: &str) {
        self.devices
            .insert(format!("{bus}:{vid}:{pid}"), driver.to_string());
    }
}
