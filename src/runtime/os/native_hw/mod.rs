//! Native hardware backends — real audio (cpal) and USB (serialport + nusb).

#[cfg(all(not(target_arch = "wasm32"), feature = "hw"))]
pub mod audio;
#[cfg(all(not(target_arch = "wasm32"), feature = "hw"))]
pub mod usb;

#[cfg(all(not(target_arch = "wasm32"), feature = "hw"))]
pub use audio::AudioBackend;
#[cfg(all(not(target_arch = "wasm32"), feature = "hw"))]
pub use usb::UsbBackend;

/// True when built with `hw` on native and not disabled via env.
pub fn enabled() -> bool {
    #[cfg(all(not(target_arch = "wasm32"), feature = "hw"))]
    {
        std::env::var("KABOOTAR_HW")
            .map(|v| v != "0" && !v.eq_ignore_ascii_case("false"))
            .unwrap_or(true)
    }
    #[cfg(not(all(not(target_arch = "wasm32"), feature = "hw")))]
    {
        false
    }
}

pub fn info_map() -> std::collections::HashMap<String, String> {
    let mut m = std::collections::HashMap::new();
    m.insert("hw_feature".into(), cfg!(feature = "hw").to_string());
    m.insert("hw_enabled".into(), enabled().to_string());
    #[cfg(all(not(target_arch = "wasm32"), feature = "hw"))]
    {
        m.insert("audio_backend".into(), "cpal".into());
        m.insert("usb_backend".into(), "serialport+hidapi+nusb".into());
        m.insert("net_backend".into(), "host-ifaces".into());
    }
    #[cfg(not(all(not(target_arch = "wasm32"), feature = "hw")))]
    {
        m.insert("audio_backend".into(), "simulated".into());
        m.insert("usb_backend".into(), "simulated".into());
        m.insert("net_backend".into(), "simulated".into());
    }
    m
}

/// No-op backend when `hw` feature is off.
#[cfg(not(all(not(target_arch = "wasm32"), feature = "hw")))]
#[derive(Debug, Default)]
pub struct AudioBackend;

#[cfg(not(all(not(target_arch = "wasm32"), feature = "hw")))]
impl AudioBackend {
    pub fn is_available(&self) -> bool {
        false
    }
    pub fn last_error(&self) -> Option<&str> {
        Some("rebuild with --features hw")
    }
    pub fn rescan(&mut self) {}
    pub fn device_infos(&self) -> Vec<super::drivers::audio::AudioDeviceInfo> {
        Vec::new()
    }
    pub fn is_host_device(&self, _: &str) -> bool {
        false
    }
    pub fn open(&mut self, _: &str, _: Option<u8>, _: Option<u32>) -> Result<u64, String> {
        Err("native audio requires --features hw".into())
    }
    pub fn close(&mut self, _: u64) -> bool {
        false
    }
    pub fn write_pcm(&mut self, _: u64, _: &[i16]) -> Result<usize, String> {
        Err("native audio requires --features hw".into())
    }
    pub fn read_pcm(&mut self, _: u64, _: usize) -> Result<Vec<i16>, String> {
        Err("native audio requires --features hw".into())
    }
}

#[cfg(not(all(not(target_arch = "wasm32"), feature = "hw")))]
#[derive(Debug, Default)]
pub struct UsbBackend;

#[cfg(not(all(not(target_arch = "wasm32"), feature = "hw")))]
impl UsbBackend {
    pub fn is_available(&self) -> bool {
        false
    }
    pub fn last_error(&self) -> Option<&str> {
        Some("rebuild with --features hw")
    }
    pub fn rescan(&mut self) {}
    pub fn device_infos(&self) -> Vec<super::drivers::usb::UsbDeviceInfo> {
        Vec::new()
    }
    pub fn is_host_device(&self, _: &str) -> bool {
        false
    }
    pub fn is_serial_device(&self, _: &str) -> bool {
        false
    }
    pub fn is_hid_device(&self, _: &str) -> bool {
        false
    }

    pub fn is_openable(&self, _: &str) -> bool {
        false
    }

    pub fn stats(&self) -> (usize, usize, usize) {
        (0, 0, 0)
    }

    pub fn open(&mut self, _: &str) -> Result<u64, String> {
        Err("native usb requires --features hw".into())
    }
    pub fn close(&mut self, _: u64) -> bool {
        false
    }
    pub fn transfer(&mut self, _: u64, _: &str, _: &[u8]) -> Result<Vec<u8>, String> {
        Err("native usb requires --features hw".into())
    }
}
