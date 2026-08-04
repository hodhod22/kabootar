//! Kabootar OS display server — monitors, vsync, compositor layers.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

/// Process-wide vsync mode (`fifo` | `immediate`) so shell present can read it
/// without an `OsHandle`.
fn vsync_slot() -> &'static Mutex<String> {
    static VSYNC: OnceLock<Mutex<String>> = OnceLock::new();
    VSYNC.get_or_init(|| Mutex::new("fifo".into()))
}

/// Current DisplayServer vsync mode (`"fifo"` or `"immediate"`).
pub fn display_vsync_mode() -> String {
    vsync_slot()
        .lock()
        .map(|g| g.clone())
        .unwrap_or_else(|_| "fifo".into())
}

#[derive(Debug, Clone)]
pub struct DisplaySurface {
    pub window_id: u64,
    pub width: i64,
    pub height: i64,
    pub title: String,
    pub last_frame_bytes: usize,
}

#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub id: u64,
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
    pub primary: bool,
}

#[derive(Debug, Clone)]
pub struct CompositorLayer {
    pub id: u64,
    pub window_id: u64,
    pub blur: u32,
    pub opacity: f64,
}

pub struct DisplayServer {
    next_id: AtomicU64,
    next_layer: AtomicU64,
    surfaces: HashMap<u64, DisplaySurface>,
    monitors: Vec<MonitorInfo>,
    vsync: String,
    layers: Vec<CompositorLayer>,
}

impl Default for DisplayServer {
    fn default() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            next_layer: AtomicU64::new(1),
            surfaces: HashMap::new(),
            monitors: vec![
                MonitorInfo {
                    id: 1,
                    x: 0,
                    y: 0,
                    width: 1920,
                    height: 1080,
                    primary: true,
                },
                MonitorInfo {
                    id: 2,
                    x: 1920,
                    y: 0,
                    width: 1280,
                    height: 720,
                    primary: false,
                },
            ],
            vsync: "fifo".into(),
            layers: Vec::new(),
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

    pub fn monitors(&self) -> &[MonitorInfo] {
        &self.monitors
    }

    pub fn set_vsync(&mut self, mode: &str) -> Result<String, String> {
        let m = mode.to_ascii_lowercase();
        if m != "fifo" && m != "immediate" {
            return Err("os_display_vsync expects fifo or immediate".into());
        }
        if let Ok(mut g) = vsync_slot().lock() {
            *g = m.clone();
        }
        self.vsync = m;
        Ok(self.vsync.clone())
    }

    pub fn vsync(&self) -> &str {
        &self.vsync
    }

    pub fn add_layer(&mut self, window_id: u64, blur: u32, opacity: f64) -> u64 {
        let id = self.next_layer.fetch_add(1, Ordering::SeqCst);
        self.layers.push(CompositorLayer {
            id,
            window_id,
            blur,
            opacity: opacity.clamp(0.0, 1.0),
        });
        id
    }

    pub fn layers(&self) -> &[CompositorLayer] {
        &self.layers
    }

    /// Soft acrylic preview: estimate blurred underlay bytes for a layer.
    pub fn acrylic_preview_bytes(&self, layer_id: u64) -> Option<usize> {
        let layer = self.layers.iter().find(|l| l.id == layer_id)?;
        let surface = self.surfaces.values().find(|s| s.window_id == layer.window_id)?;
        let base = if surface.last_frame_bytes > 0 {
            surface.last_frame_bytes
        } else {
            (surface.width.max(1) * surface.height.max(1) * 4) as usize
        };
        // Blur expands working set slightly; opacity scales reported composite cost.
        let blur_cost = (layer.blur as usize).saturating_mul(64);
        let scaled = ((base + blur_cost) as f64 * layer.opacity.max(0.1)) as usize;
        Some(scaled)
    }
}
