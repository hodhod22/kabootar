//! Game frame loop — `requestAnimationFrame` / `game_tick`.

use crate::bytecode::call_value;
use crate::value::{unix_ms_now, Environment, Value};
use std::cell::RefCell;
use std::collections::{HashSet, VecDeque};

thread_local! {
    static NEXT_FRAME_ID: RefCell<u64> = RefCell::new(1);
    static FRAME_STATE: RefCell<FrameState> = RefCell::new(FrameState::default());
}

struct FrameState {
    pending: VecDeque<(u64, Value)>,
    cancelled: HashSet<u64>,
    last_tick_ms: u64,
    frame_count: u64,
}

impl Default for FrameState {
    fn default() -> Self {
        Self {
            pending: VecDeque::new(),
            cancelled: HashSet::new(),
            last_tick_ms: unix_ms_now(),
            frame_count: 0,
        }
    }
}

pub fn request_frame(callback: Value) -> u64 {
    NEXT_FRAME_ID.with(|n| {
        let mut id = n.borrow_mut();
        let current = *id;
        *id = id.saturating_add(1);
        FRAME_STATE.with(|s| {
            let mut s = s.borrow_mut();
            s.cancelled.remove(&current);
            s.pending.push_back((current, callback));
        });
        current
    })
}

pub fn cancel_frame(id: u64) {
    FRAME_STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.cancelled.insert(id);
        s.pending.retain(|(fid, _)| *fid != id);
    });
}

pub fn has_pending_frames() -> bool {
    FRAME_STATE.with(|s| !s.borrow().pending.is_empty())
}

pub fn tick(env: &mut Environment) -> Result<Value, String> {
    crate::runtime::stdlib::weak::gc_frame_begin();

    let (delta_ms, frame_no, time_ms, callbacks) = FRAME_STATE.with(|s| {
        let mut s = s.borrow_mut();
        let now = unix_ms_now();
        let delta = now.saturating_sub(s.last_tick_ms);
        s.last_tick_ms = now;
        s.frame_count = s.frame_count.saturating_add(1);
        let mut callbacks = Vec::new();
        while let Some((id, cb)) = s.pending.pop_front() {
            if s.cancelled.remove(&id) {
                continue;
            }
            callbacks.push(cb);
        }
        (delta, s.frame_count, now, callbacks)
    });

    let delta_val = Value::Float(delta_ms as f64);
    for cb in callbacks {
        call_value(cb, vec![delta_val.clone()], &[], &[], &[], &[], env)?;
    }

    // P3: soft GC budget — sweep if this frame allocated heavily.
    let _ = crate::runtime::stdlib::weak::gc_frame_maybe_sweep(env)?;

    Ok(frame_info_object(delta_ms, frame_no, time_ms))
}

pub fn frame_info_object(delta_ms: u64, frame: u64, time_ms: u64) -> Value {
    let mut m = std::collections::HashMap::new();
    m.insert("delta_ms".into(), Value::Float(delta_ms as f64));
    m.insert("frame".into(), Value::Number(frame as i64));
    m.insert("time_ms".into(), Value::Number(time_ms as i64));
    Value::Object(m)
}

pub fn reset_for_tests() {
    FRAME_STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.pending.clear();
        s.cancelled.clear();
        s.frame_count = 0;
        s.last_tick_ms = unix_ms_now();
    });
    NEXT_FRAME_ID.with(|n| *n.borrow_mut() = 1);
    crate::runtime::stdlib::weak::gc_frame_reset_for_tests();
}
