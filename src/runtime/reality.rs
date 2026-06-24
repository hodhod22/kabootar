//! Honest capability tiers — marketing vs reality (punkt 3).

use crate::modules::list_builtins;
use crate::value::{Environment, Value};
use std::collections::HashMap;

/// Tier key: what actually runs today.
#[derive(Clone, Copy)]
struct FeatureTruth {
    id: &'static str,
    area: &'static str,
    tier: &'static str,
    claim: &'static str,
    reality: &'static str,
}

const FEATURES: &[FeatureTruth] = &[
    FeatureTruth {
        id: "compiler",
        area: "toolchain",
        tier: "partial",
        claim: "Self-hosting Kabootar compiler",
        reality: "Compiler and VM are implemented in Rust today",
    },
    FeatureTruth {
        id: "stdlib",
        area: "language",
        tier: "native",
        claim: "JSON, Map/Set, reduce, regex",
        reality: "Native stdlib registered at startup; import \"std\" for wrappers",
    },
    FeatureTruth {
        id: "bytecode",
        area: "language",
        tier: "native",
        claim: "Fast bytecode execution",
        reality: "Bytecode VM + .kbc cache; AST fallback for advanced syntax",
    },
    FeatureTruth {
        id: "sql",
        area: "runtime",
        tier: "native",
        claim: "In-process SQL database",
        reality: "Real planner, WAL, MVCC inside the Kabootar process",
    },
    FeatureTruth {
        id: "http",
        area: "runtime",
        tier: "native",
        claim: "HTTP server and client",
        reality: "Native TCP/TLS on desktop; in-process routes on serve",
    },
    FeatureTruth {
        id: "os_vfs",
        area: "os",
        tier: "sandbox",
        claim: "Kabootar OS filesystem",
        reality: "Virtual FS in-process — not a host kernel replacement",
    },
    FeatureTruth {
        id: "os_display",
        area: "os",
        tier: "sandbox",
        claim: "Display server and compositor",
        reality: "Framebuffer + wgpu path inside Kabootar runtime",
    },
    FeatureTruth {
        id: "os_drivers",
        area: "os",
        tier: "partial",
        claim: "GPU, net, USB, audio drivers",
        reality: "Driver model + native hw with --features hw; many paths simulated",
    },
    FeatureTruth {
        id: "os_sauce_ai",
        area: "os",
        tier: "sandbox",
        claim: "AI prefetch and context menus",
        reality: "In-memory launch heuristics and scheduler enqueue — no cloud ML",
    },
    FeatureTruth {
        id: "os_sauce_setup",
        area: "os",
        tier: "sandbox",
        claim: "Zero-touch NFC setup",
        reality: "Timed setup simulation with NFC token bump API",
    },
    FeatureTruth {
        id: "os_sauce_recovery",
        area: "os",
        tier: "sandbox",
        claim: "2-second golden restore",
        reality: "VFS snapshot restore with measured latency — not bare-metal imaging",
    },
    FeatureTruth {
        id: "os_sauce_compat",
        area: "os",
        tier: "stub",
        claim: "99% Windows/Linux app compatibility",
        reality: "Syscall translation table — does not run foreign binaries",
    },
    FeatureTruth {
        id: "os_sauce_energy",
        area: "os",
        tier: "sandbox",
        claim: "Forever battery scheduling",
        reality: "Repaint gating and deferred job queue in sandbox",
    },
    FeatureTruth {
        id: "os_sauce_privacy",
        area: "os",
        tier: "sandbox",
        claim: "Zero telemetry by design",
        reality: "Telemetry off by default; privacy switch is in-process state",
    },
    FeatureTruth {
        id: "browser_kv8",
        area: "browser",
        tier: "partial",
        claim: "Kabootar browser engine",
        reality: "Kv8 layout + kdom; not Chromium/WebKit parity",
    },
    FeatureTruth {
        id: "browser_webrtc",
        area: "browser",
        tier: "stub",
        claim: "WebRTC calls",
        reality: "ICE config + SDP stub — no full media stack",
    },
    FeatureTruth {
        id: "browser_webgl",
        area: "browser",
        tier: "partial",
        claim: "WebGL 2.0",
        reality: "wgpu buffers and shaders — subset of WebGL API",
    },
    FeatureTruth {
        id: "crypto_pq",
        area: "security",
        tier: "stub",
        claim: "Post-quantum Kyber/Dilithium",
        reality: "API stubs for encapsulate/sign — not production PQ crypto",
    },
    FeatureTruth {
        id: "ecosystem",
        area: "ecosystem",
        tier: "early",
        claim: "Large package ecosystem",
        reality: "Built-in modules + local registry; use ecosystem_info() and registry_seed()",
    },
    FeatureTruth {
        id: "js_parity",
        area: "language",
        tier: "partial",
        claim: "All of JavaScript",
        reality: "JS-like core minus footguns; not full ECMAScript/DOM/Node",
    },
];

fn feature_entry(f: &FeatureTruth) -> Value {
    let mut m = HashMap::new();
    m.insert("id".into(), Value::String(f.id.into()));
    m.insert("area".into(), Value::String(f.area.into()));
    m.insert("tier".into(), Value::String(f.tier.into()));
    m.insert("claim".into(), Value::String(f.claim.into()));
    m.insert("reality".into(), Value::String(f.reality.into()));
    Value::Object(m)
}

fn find_feature(id: &str) -> Option<&'static FeatureTruth> {
    let key = id.to_ascii_lowercase();
    FEATURES.iter().find(|f| f.id.eq_ignore_ascii_case(&key))
}

fn kabootar_reality_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let builtins: Vec<Value> = list_builtins()
        .into_iter()
        .map(|s| Value::String(s.into()))
        .collect();
    let features: Vec<Value> = FEATURES.iter().map(feature_entry).collect();
    let mut root = HashMap::new();
    root.insert("version".into(), Value::String(env!("CARGO_PKG_VERSION").into()));
    root.insert("self_hosting".into(), Value::Bool(false));
    root.insert("compiler_host".into(), Value::String("rust".into()));
    root.insert(
        "ecosystem_stage".into(),
        Value::String("early".into()),
    );
    root.insert(
        "builtin_modules".into(),
        Value::Array(builtins),
    );
    root.insert("features".into(), Value::Array(features));
    root.insert(
        "tier_legend".into(),
        Value::Object(
            [
                ("native", "Runs for real inside Kabootar process"),
                ("sandbox", "Modeled API — simulated OS/browser behavior"),
                ("partial", "Works for common cases — gaps remain"),
                ("stub", "API only — not production behavior"),
                ("early", "Exists but not mature at ecosystem scale"),
            ]
            .into_iter()
            .map(|(k, v)| (k.into(), Value::String(v.into())))
            .collect(),
        ),
    );
    root.insert(
        "summary".into(),
        Value::String(
            "Kabootar is a broad fullstack runtime, not a drop-in replacement for Windows, Chrome, or Rust. Use kabootar_reality() and feature_tier(id) before trusting marketing claims.".into(),
        ),
    );
    Ok(Value::Object(root))
}

fn feature_tier_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let id = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("feature_tier(id)".into()),
    };
    let f = find_feature(id).ok_or_else(|| format!("unknown feature id: {id}"))?;
    Ok(feature_entry(f))
}

pub fn reality_globals(env: &mut Environment) {
    env.set(
        "kabootar_reality".to_string(),
        Value::NativeFunction(kabootar_reality_native),
    );
    env.set(
        "feature_tier".to_string(),
        Value::NativeFunction(feature_tier_native),
    );
}

pub fn sauce_strategy_honesty() -> Vec<(String, String, String)> {
    vec![
        (
            "s1_ai_prefetch".into(),
            "sandbox".into(),
            "Launch heuristics + scheduler — not cloud AI".into(),
        ),
        (
            "s2_setup_nfc".into(),
            "sandbox".into(),
            "Timed setup flow with NFC token API".into(),
        ),
        (
            "s3_golden_restore".into(),
            "sandbox".into(),
            "VFS snapshot restore with latency measurement".into(),
        ),
        (
            "s4_seamless_mesh".into(),
            "sandbox".into(),
            "In-memory clipboard/pair counters".into(),
        ),
        (
            "s5_energy".into(),
            "sandbox".into(),
            "Repaint gating and deferred jobs".into(),
        ),
        (
            "s6_haptic".into(),
            "sandbox".into(),
            "Event counter — no device motors".into(),
        ),
        (
            "s7_compat".into(),
            "stub".into(),
            "Syscall table — does not execute foreign apps".into(),
        ),
        (
            "s8_privacy".into(),
            "sandbox".into(),
            "In-process privacy switch and telemetry gate".into(),
        ),
        (
            "s9_updates".into(),
            "sandbox".into(),
            "Partition swap timing simulation".into(),
        ),
    ]
}
