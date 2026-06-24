//! Dual-layer platform — host (real OS/DOM/browser) vs Kabootar-native stack.
//!
//! Kabootar apps can target:
//! - **Host** — existing OS, browser DOM, and Chrome-like APIs via adapters
//! - **Kabootar** — `os`, `kdom`, `kbrowser` (own kernel, DOM, browser)
//! - **Hybrid** — both layers registered; app chooses per call site

use crate::value::{Environment, Value};
use std::collections::HashMap;

/// Which runtime stack the app prefers for new UI/OS work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeLayer {
    Host,
    Kabootar,
    Hybrid,
}

impl RuntimeLayer {
    pub fn as_str(self) -> &'static str {
        match self {
            RuntimeLayer::Host => "host",
            RuntimeLayer::Kabootar => "kabootar",
            RuntimeLayer::Hybrid => "hybrid",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "host" => Some(RuntimeLayer::Host),
            "kabootar" | "k" => Some(RuntimeLayer::Kabootar),
            "hybrid" | "both" => Some(RuntimeLayer::Hybrid),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlatformState {
    pub active_layer: RuntimeLayer,
}

impl Default for PlatformState {
    fn default() -> Self {
        Self {
            active_layer: RuntimeLayer::Hybrid,
        }
    }
}

impl PlatformState {
    pub fn info(&self) -> HashMap<String, Value> {
        let mut layers = HashMap::new();
        layers.insert(
            "host".into(),
            Value::Object(host_layer_desc()),
        );
        layers.insert(
            "kabootar".into(),
            Value::Object(kabootar_layer_desc()),
        );

        let mut out = HashMap::new();
        out.insert(
            "active".into(),
            Value::String(self.active_layer.as_str().into()),
        );
        out.insert("layers".into(), Value::Object(layers));
        out.insert(
            "model".into(),
            Value::String("dual-layer".into()),
        );
        out
    }
}

fn host_layer_desc() -> HashMap<String, Value> {
    let mut m = HashMap::new();
    m.insert("os".into(), Value::String("host operating system".into()));
    m.insert("dom".into(), Value::String("document / window / navigator".into()));
    m.insert(
        "browser".into(),
        Value::String("Chrome-like host browser APIs".into()),
    );
    m.insert(
        "when".into(),
        Value::String("WASM in browser, or hybrid apps using real DOM".into()),
    );
    m
}

fn kabootar_layer_desc() -> HashMap<String, Value> {
    let mut m = HashMap::new();
    m.insert("os".into(), Value::String("kabootar-kernel + VFS + windows".into()));
    m.insert("dom".into(), Value::String("kdom + KML".into()));
    m.insert(
        "browser".into(),
        Value::String("kbrowser — tabs, history, KDOM viewport".into()),
    );
    m.insert(
        "when".into(),
        Value::String("native apps, SSR, future Kabootar desktop".into()),
    );
    m
}

fn platform_info_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let state = get_platform(env)?;
    Ok(Value::Object(state.info()))
}

fn platform_layer_native(_args: &[Value], env: &mut Environment) -> Result<Value, String> {
    Ok(Value::String(get_platform(env)?.active_layer.as_str().into()))
}

fn platform_use_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let layer = args
        .first()
        .and_then(|v| match v {
            Value::String(s) => RuntimeLayer::from_str(s),
            _ => None,
        })
        .ok_or("platform_use() expects \"host\", \"kabootar\", or \"hybrid\"")?;
    set_platform_layer(env, layer)?;
    Ok(Value::String(layer.as_str().into()))
}

fn get_platform(env: &Environment) -> Result<PlatformState, String> {
    let v = env
        .get("__platform")
        .ok_or("Platform state not initialized")?;
    let Value::Object(map) = v else {
        return Err("Platform state corrupted".into());
    };
    let active = map
        .get("active")
        .and_then(|v| match v {
            Value::String(s) => RuntimeLayer::from_str(s),
            _ => None,
        })
        .unwrap_or(RuntimeLayer::Hybrid);
    Ok(PlatformState { active_layer: active })
}

fn set_platform_layer(env: &mut Environment, layer: RuntimeLayer) -> Result<(), String> {
    let mut map = HashMap::new();
    map.insert("active".into(), Value::String(layer.as_str().into()));
    env.set("__platform".into(), Value::Object(map));
    Ok(())
}

pub fn platform_globals(env: &mut Environment) {
    let mut map = HashMap::new();
    map.insert(
        "active".into(),
        Value::String(RuntimeLayer::Hybrid.as_str().into()),
    );
    env.set("__platform".into(), Value::Object(map));
    env.set(
        "platform_info".into(),
        Value::NativeFunction(platform_info_native),
    );
    env.set(
        "platform_layer".into(),
        Value::NativeFunction(platform_layer_native),
    );
    env.set(
        "platform_use".into(),
        Value::NativeFunction(platform_use_native),
    );
}
