//! Driver framework — standardized WDF-like registration.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DriverRegistration {
    pub id: u64,
    pub name: String,
    pub version: String,
    pub loaded: bool,
}

pub struct DriverFramework {
    next_id: u64,
    drivers: HashMap<u64, DriverRegistration>,
}

impl Default for DriverFramework {
    fn default() -> Self {
        Self {
            next_id: 1,
            drivers: HashMap::new(),
        }
    }
}

impl DriverFramework {
    pub fn register(&mut self, name: &str, version: &str) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.drivers.insert(
            id,
            DriverRegistration {
                id,
                name: name.to_string(),
                version: version.to_string(),
                loaded: true,
            },
        );
        id
    }

    pub fn unregister(&mut self, id: u64) -> bool {
        if let Some(d) = self.drivers.get_mut(&id) {
            d.loaded = false;
            true
        } else {
            false
        }
    }

    pub fn list(&self) -> Vec<DriverRegistration> {
        let mut v: Vec<_> = self.drivers.values().cloned().collect();
        v.sort_by_key(|d| d.id);
        v
    }
}
