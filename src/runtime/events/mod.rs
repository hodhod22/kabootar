//! Kabootar event loop — hit-testing, dispatch, handler queue (layer 2).

use crate::runtime::render::RenderLayer;
use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone)]
pub struct KabootarEvent {
    pub node_id: u64,
    pub event_type: String,
    pub handler: String,
    pub x: f64,
    pub y: f64,
}

static BUS: OnceLock<Mutex<EventBus>> = OnceLock::new();

fn bus() -> &'static Mutex<EventBus> {
    BUS.get_or_init(|| Mutex::new(EventBus::default()))
}

#[derive(Debug, Default)]
struct EventBus {
    queue: VecDeque<KabootarEvent>,
}

pub fn hit_test(layers: &[RenderLayer], x: f64, y: f64) -> Option<u64> {
    let mut best: Option<(i32, u64)> = None;
    for layer in layers {
        if x >= layer.x && x < layer.x + layer.w && y >= layer.y && y < layer.y + layer.h {
            let z = layer.z;
            match best {
                Some((bz, _)) if bz >= z => {}
                _ => best = Some((z, layer.node_id)),
            }
        }
    }
    best.map(|(_, id)| id)
}

pub fn enqueue(event: KabootarEvent) {
    if let Ok(mut g) = bus().lock() {
        g.queue.push_back(event);
    }
}

pub fn drain_events() -> Vec<KabootarEvent> {
    bus()
        .lock()
        .map(|mut g| g.queue.drain(..).collect())
        .unwrap_or_default()
}

pub fn pending_count() -> usize {
    bus().lock().map(|g| g.queue.len()).unwrap_or(0)
}

pub fn clear_events() {
    if let Ok(mut g) = bus().lock() {
        g.queue.clear();
    }
}
