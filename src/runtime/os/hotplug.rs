//! Kabootar OS hotplug bus — device add/remove events for browser and apps.

use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone)]
pub struct HotplugEvent {
    pub action: String,
    pub device_id: String,
    pub kind: String,
    pub name: String,
    pub vendor: String,
}

static BUS: OnceLock<Mutex<HotplugBus>> = OnceLock::new();

fn bus() -> &'static Mutex<HotplugBus> {
    BUS.get_or_init(|| Mutex::new(HotplugBus::default()))
}

#[derive(Debug, Default)]
struct HotplugBus {
    queue: VecDeque<HotplugEvent>,
}

pub fn emit_add(device_id: &str, kind: &str, name: &str, vendor: &str) {
    enqueue(HotplugEvent {
        action: "add".into(),
        device_id: device_id.into(),
        kind: kind.into(),
        name: name.into(),
        vendor: vendor.into(),
    });
}

pub fn emit_remove(device_id: &str, kind: &str) {
    enqueue(HotplugEvent {
        action: "remove".into(),
        device_id: device_id.into(),
        kind: kind.into(),
        name: String::new(),
        vendor: String::new(),
    });
}

fn enqueue(ev: HotplugEvent) {
    if let Ok(mut g) = bus().lock() {
        g.queue.push_back(ev);
    }
}

pub fn drain() -> Vec<HotplugEvent> {
    bus()
        .lock()
        .map(|mut g| g.queue.drain(..).collect())
        .unwrap_or_default()
}

pub fn pending_count() -> usize {
    bus().lock().map(|g| g.queue.len()).unwrap_or(0)
}

pub fn clear() {
    if let Ok(mut g) = bus().lock() {
        g.queue.clear();
    }
}
