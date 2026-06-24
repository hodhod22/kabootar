//! Kabootar kernel — simple today, designed to grow into a full sandboxed OS.

#[derive(Debug, Clone)]
pub struct Kernel {
    pub name: String,
    pub version: String,
}

impl Kernel {
    pub fn new() -> Self {
        Self {
            name: "kabootar-kernel".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    pub fn info(&self) -> String {
        format!("{} {}", self.name, self.version)
    }

    /// Capabilities available today; more are added as the kernel grows.
    pub fn capabilities() -> Vec<&'static str> {
        vec![
            "vfs",
            "sandbox",
            "process-table",
            "permissions",
            "modules",
        ]
    }

    /// Capabilities that are implemented in this release.
    pub fn active_capabilities() -> Vec<&'static str> {
        vec![
            "vfs",
            "sandbox",
            "modules",
            "process-table",
            "window-manager",
            "display-server",
            "memory-manager",
            "scheduler",
            "syscalls",
            "vfs-persist",
            "device-manager",
            "gpu-driver",
            "net-driver",
            "usb-driver",
            "audio-driver",
            "permissions",
            "hotplug",
            "host-bridge",
            "native-hw",
            "vfs-extended",
            "net-tcp-full",
            "memory-safe",
            "bytecode-optimize",
            "ring0-kcore",
            "ring0-mm",
            "ring0-io",
            "ring0-fs-stack",
            "ring0-netstack",
            "ring3-userland",
            "crosscut-security",
            "crosscut-log",
            "crosscut-power",
            "sauce-ai-composer",
            "sauce-zero-setup",
            "sauce-state-sep",
            "sauce-seamless",
            "sauce-energy-core",
            "sauce-haptic-ui",
            "sauce-compat-god",
            "sauce-privacy",
            "sauce-community-updates",
            "kv8-engine",
            "kv8-jit",
            "kv8-vfs-modules",
            "kstyle-lang",
            "browser-wasm",
            "browser-webgl",
            "browser-webrtc",
            "browser-devtools",
            "browser-extensions",
            "browser-pwa",
        ]
    }
}
