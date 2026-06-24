//! Strategy 3 — State separation: OS / apps / data partitions + golden image recovery.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Partition {
    Os,
    Apps,
    Data,
}

pub struct StateSeparation {
    os_version: String,
    apps_count: u32,
    data_bytes: u64,
    golden_image: String,
    last_restore_ms: u64,
}

impl Default for StateSeparation {
    fn default() -> Self {
        Self {
            os_version: "kabootar-golden-2.1".into(),
            apps_count: 0,
            data_bytes: 0,
            golden_image: "/efi/golden.img".into(),
            last_restore_ms: 0,
        }
    }
}

impl StateSeparation {
    pub fn register_app(&mut self) {
        self.apps_count += 1;
    }

    pub fn write_data(&mut self, bytes: u64) {
        self.data_bytes += bytes;
    }

    pub fn recovery_combo(&self) -> &'static str {
        "vol_up+power"
    }

    pub fn golden_restore(&mut self, elapsed_ms: u64) -> u64 {
        self.os_version = "kabootar-golden-2.1".into();
        self.last_restore_ms = elapsed_ms.max(1).min(2000);
        self.last_restore_ms
    }

    pub fn partition_stats(&self) -> (String, u32, u64) {
        (self.os_version.clone(), self.apps_count, self.data_bytes)
    }

    pub fn golden_path(&self) -> &str {
        &self.golden_image
    }
}
