//! Game input — keyboard and pointer state for native shell + tests.

use crate::value::Value;
use std::cell::RefCell;
use std::collections::{HashSet, VecDeque};

#[derive(Debug, Clone, Default)]
struct PointerState {
    x: f64,
    y: f64,
    down: bool,
}

#[derive(Debug, Default)]
struct InputState {
    keys_down: HashSet<String>,
    keys_pressed: VecDeque<String>,
    keys_released: VecDeque<String>,
    pointer: PointerState,
}

thread_local! {
    static INPUT: RefCell<InputState> = RefCell::new(InputState::default());
}

pub fn key_down(key: &str) {
    INPUT.with(|s| {
        let mut s = s.borrow_mut();
        let k = normalize_key(key);
        if s.keys_down.insert(k.clone()) {
            s.keys_pressed.push_back(k);
        }
    });
}

pub fn key_up(key: &str) {
    INPUT.with(|s| {
        let mut s = s.borrow_mut();
        let k = normalize_key(key);
        if s.keys_down.remove(&k) {
            s.keys_released.push_back(k);
        }
    });
}

pub fn pointer_move(x: f64, y: f64) {
    INPUT.with(|s| {
        let mut s = s.borrow_mut();
        s.pointer.x = x;
        s.pointer.y = y;
    });
}

pub fn pointer_down(x: f64, y: f64) {
    INPUT.with(|s| {
        let mut s = s.borrow_mut();
        s.pointer.x = x;
        s.pointer.y = y;
        s.pointer.down = true;
    });
}

pub fn pointer_up(x: f64, y: f64) {
    INPUT.with(|s| {
        let mut s = s.borrow_mut();
        s.pointer.x = x;
        s.pointer.y = y;
        s.pointer.down = false;
    });
}

pub fn is_down(key: &str) -> bool {
    INPUT.with(|s| s.borrow().keys_down.contains(&normalize_key(key)))
}

pub fn poll() -> Value {
    INPUT.with(|s| {
        let mut s = s.borrow_mut();
        let pressed: Vec<Value> = s.keys_pressed.drain(..).map(Value::String).collect();
        let released: Vec<Value> = s.keys_released.drain(..).map(Value::String).collect();
        let down: Vec<Value> = s.keys_down.iter().cloned().map(Value::String).collect();
        let mut pm = std::collections::HashMap::new();
        pm.insert("x".into(), Value::Float(s.pointer.x));
        pm.insert("y".into(), Value::Float(s.pointer.y));
        pm.insert("down".into(), Value::Bool(s.pointer.down));
        let mut m = std::collections::HashMap::new();
        m.insert("pressed".into(), Value::from_array(pressed));
        m.insert("released".into(), Value::from_array(released));
        m.insert("down".into(), Value::from_array(down));
        m.insert("pointer".into(), Value::from_object(pm));
        Value::from_object(m)
    })
}

pub fn reset_for_tests() {
    INPUT.with(|s| {
        let mut s = s.borrow_mut();
        s.keys_down.clear();
        s.keys_pressed.clear();
        s.keys_released.clear();
        s.pointer = PointerState::default();
    });
}

fn normalize_key(key: &str) -> String {
    match key {
        " " => "Space".into(),
        other => other.to_string(),
    }
}
