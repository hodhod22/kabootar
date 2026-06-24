//! Kabootar OS display server — connects OS windows to compositor frames.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone)]
pub struct DisplaySurface {
    pub window_id: u64,
    pub width: i64,
    pub height: i64,
    pub title: String,
    pub last_frame_bytes: usize,
}

pub struct DisplayServer {
    next_id: AtomicU64,
    surfaces: HashMap<u64, DisplaySurface>,
}

impl Default for DisplayServer {
    fn default() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            surfaces: HashMap::new(),
        }
    }
}

impl DisplayServer {
    pub fn register(&mut self, window_id: u64, title: &str, width: i64, height: i64) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        self.surfaces.insert(
            id,
            DisplaySurface {
                window_id,
                width,
                height,
                title: title.to_string(),
                last_frame_bytes: 0,
            },
        );
        id
    }

    pub fn present(&mut self, window_id: u64, frame_bytes: usize) -> bool {
        for s in self.surfaces.values_mut() {
            if s.window_id == window_id {
                s.last_frame_bytes = frame_bytes;
                return true;
            }
        }
        false
    }

    pub fn list(&self) -> Vec<DisplaySurface> {
        let mut out: Vec<_> = self.surfaces.values().cloned().collect();
        out.sort_by_key(|s| s.window_id);
        out
    }
}
