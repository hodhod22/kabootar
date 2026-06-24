//! Window manager for Kabootar OS — Chrome-like windows over the native stack.

use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone)]
pub struct OsWindow {
    pub id: u64,
    pub title: String,
    pub width: i64,
    pub height: i64,
    pub focused: bool,
    pub browser_tab_id: Option<u64>,
}

pub struct WindowManager {
    next_id: AtomicU64,
    windows: Vec<OsWindow>,
}

impl Default for WindowManager {
    fn default() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            windows: Vec::new(),
        }
    }
}

impl WindowManager {
    pub fn create(&mut self, title: &str, width: i64, height: i64) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        for w in &mut self.windows {
            w.focused = false;
        }
        self.windows.push(OsWindow {
            id,
            title: title.to_string(),
            width,
            height,
            focused: true,
            browser_tab_id: None,
        });
        id
    }

    pub fn list(&self) -> Vec<OsWindow> {
        self.windows.clone()
    }

    pub fn bind_tab(&mut self, window_id: u64, tab_id: u64) -> bool {
        if let Some(w) = self.windows.iter_mut().find(|w| w.id == window_id) {
            w.browser_tab_id = Some(tab_id);
            return true;
        }
        false
    }
}
