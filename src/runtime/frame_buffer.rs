//! Global compositor frame buffer — bridges Kabootar layer to host browser (WASM).

use crate::runtime::render::CompositorFrame;
use std::sync::{Mutex, OnceLock};

static FRAME: OnceLock<Mutex<Option<CompositorFrame>>> = OnceLock::new();

fn slot() -> &'static Mutex<Option<CompositorFrame>> {
    FRAME.get_or_init(|| Mutex::new(None))
}

pub fn publish_frame(frame: CompositorFrame) {
    if let Ok(mut g) = slot().lock() {
        *g = Some(frame);
    }
}

pub fn last_frame() -> Option<CompositorFrame> {
    slot().lock().ok().and_then(|g| g.clone())
}

pub fn last_frame_html() -> Option<String> {
    last_frame().map(|f| f.html)
}

pub fn last_frame_text() -> Option<String> {
    last_frame().map(|f| f.text_preview)
}

pub fn last_frame_pixels() -> Option<(i64, i64, Vec<u8>)> {
    last_frame().map(|f| (f.width, f.height, f.pixels_rgba))
}

pub fn clear_frame() {
    if let Ok(mut g) = slot().lock() {
        *g = None;
    }
}

/// Publish raw RGBA pixels (e.g. from WebGL).
pub fn publish_pixels(width: f64, height: f64, pixels: Vec<u8>) {
    use crate::runtime::render::CompositorFrame;
    publish_frame(CompositorFrame {
        width: width as i64,
        height: height as i64,
        html: String::new(),
        text_preview: "webgl".into(),
        node_count: 0,
        layers: vec![],
        pixels_rgba: pixels,
        backend: "webgl".into(),
        gpu_handle: None,
    });
}
