//! Executive — Ring 0 system services (I/O manager, object manager).

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct KernelObject {
    pub handle: u64,
    pub kind: String,
    pub ref_count: u32,
}

pub struct Executive {
    next_handle: u64,
    objects: HashMap<u64, KernelObject>,
    io_requests: u64,
}

impl Default for Executive {
    fn default() -> Self {
        Self {
            next_handle: 1,
            objects: HashMap::new(),
            io_requests: 0,
        }
    }
}

impl Executive {
    pub fn create_object(&mut self, kind: &str) -> u64 {
        let h = self.next_handle;
        self.next_handle += 1;
        self.objects.insert(
            h,
            KernelObject {
                handle: h,
                kind: kind.to_string(),
                ref_count: 1,
            },
        );
        h
    }

    pub fn io_request(&mut self) -> u64 {
        self.io_requests += 1;
        self.io_requests
    }

    pub fn object_count(&self) -> usize {
        self.objects.len()
    }
}
