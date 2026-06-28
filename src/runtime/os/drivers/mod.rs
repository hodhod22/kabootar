//! Kabootar OS device manager — driver registry for GPU, network, USB, and audio.

pub mod audio;
mod gpu;
mod net;
pub mod usb;

pub use audio::AudioDriver;
pub use gpu::{GpuDriver, GpuDriverInfo};
pub use net::{NetDriver, NetInterface};
pub use usb::{UsbClass, UsbDriver};

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::runtime::os::host_bridge::HostBridge;
use crate::runtime::os::hotplug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverKind {
    Gpu,
    Network,
    Usb,
    Audio,
}

impl DriverKind {
    pub fn as_str(self) -> &'static str {
        match self {
            DriverKind::Gpu => "gpu",
            DriverKind::Network => "net",
            DriverKind::Usb => "usb",
            DriverKind::Audio => "audio",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "gpu" | "display" => Some(DriverKind::Gpu),
            "net" | "network" | "nic" => Some(DriverKind::Network),
            "usb" => Some(DriverKind::Usb),
            "audio" | "sound" => Some(DriverKind::Audio),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeviceDescriptor {
    pub id: String,
    pub kind: DriverKind,
    pub name: String,
    pub vendor: String,
    pub status: &'static str,
}

#[derive(Debug, Clone)]
pub struct DevHandle {
    pub id: u64,
    pub device_id: String,
    pub kind: DriverKind,
    pub sub_handle: Option<u64>,
}

pub struct DeviceManager {
    devices: Vec<DeviceDescriptor>,
    handles: HashMap<u64, DevHandle>,
    next_handle: AtomicU64,
    pub gpu: GpuDriver,
    pub net: NetDriver,
    pub usb: UsbDriver,
    pub audio: AudioDriver,
    host: HostBridge,
}

impl Default for DeviceManager {
    fn default() -> Self {
        let mut dm = Self {
            devices: Vec::new(),
            handles: HashMap::new(),
            next_handle: AtomicU64::new(1),
            gpu: GpuDriver::new(),
            net: NetDriver::default(),
            usb: UsbDriver::default(),
            audio: AudioDriver::default(),
            host: HostBridge::from_env(),
        };
        dm.register_builtins();
        if crate::runtime::os::native_hw::enabled() {
            dm.refresh_hw();
        }
        dm
    }
}

impl DeviceManager {
    /// Rescan host audio (cpal) and USB (serialport/nusb) devices.
    pub fn refresh_hw(&mut self) -> usize {
        self.audio.refresh_host();
        self.usb.refresh_host();
        self.devices.retain(|d| d.kind != DriverKind::Audio && d.kind != DriverKind::Usb);
        for u in self.usb.list() {
            self.devices.push(DeviceDescriptor {
                id: u.id.clone(),
                kind: DriverKind::Usb,
                name: u.product.clone(),
                vendor: u.vendor.clone(),
                status: "online",
            });
        }
        for a in self.audio.list() {
            self.devices.push(DeviceDescriptor {
                id: a.id.clone(),
                kind: DriverKind::Audio,
                name: a.name.clone(),
                vendor: if a.id.starts_with("host-audio-") {
                    "host".into()
                } else {
                    "Kabootar".into()
                },
                status: "online",
            });
        }
        self.devices
            .iter()
            .filter(|d| d.id.starts_with("host-"))
            .count()
    }

    fn register_builtins(&mut self) {
        self.devices.push(DeviceDescriptor {
            id: "gpu-0".into(),
            kind: DriverKind::Gpu,
            name: "Kabootar Display Adapter".into(),
            vendor: "Kabootar".into(),
            status: "online",
        });
        for iface in self.net.interfaces() {
            self.devices.push(DeviceDescriptor {
                id: format!("net-{}", iface.name),
                kind: DriverKind::Network,
                name: format!("NIC {}", iface.name),
                vendor: "Kabootar".into(),
                status: if iface.up { "online" } else { "offline" },
            });
        }
        for u in self.usb.list() {
            self.devices.push(DeviceDescriptor {
                id: u.id.clone(),
                kind: DriverKind::Usb,
                name: u.product.clone(),
                vendor: u.vendor.clone(),
                status: "online",
            });
        }
        for a in self.audio.list() {
            self.devices.push(DeviceDescriptor {
                id: a.id.clone(),
                kind: DriverKind::Audio,
                name: a.name.clone(),
                vendor: "Kabootar".into(),
                status: "online",
            });
        }
    }

    pub fn list_devices(&self) -> &[DeviceDescriptor] {
        &self.devices
    }

    pub fn list_by_kind(&self, kind: DriverKind) -> Vec<&DeviceDescriptor> {
        self.devices.iter().filter(|d| d.kind == kind).collect()
    }

    pub fn host_info(&self) -> std::collections::HashMap<String, String> {
        let mut m = self.host.info();
        m.extend(crate::runtime::os::native_hw::info_map());
        m.insert(
            "audio_native".into(),
            self.audio.native_available().to_string(),
        );
        m.insert("usb_native".into(), self.usb.native_available().to_string());
        let (hid, serial, nusb) = self.usb.native_stats();
        m.insert("usb_hid_count".into(), hid.to_string());
        m.insert("usb_serial_count".into(), serial.to_string());
        m.insert("usb_nusb_count".into(), nusb.to_string());
        m
    }

    pub fn hotplug_register(
        &mut self,
        vendor: &str,
        product: &str,
        class: UsbClass,
    ) -> String {
        let id = self.usb.hotplug(vendor, product, class);
        self.devices.push(DeviceDescriptor {
            id: id.clone(),
            kind: DriverKind::Usb,
            name: product.into(),
            vendor: vendor.into(),
            status: "online",
        });
        hotplug::emit_add(&id, "usb", product, vendor);
        id
    }

    pub fn hotplug_remove(&mut self, device_id: &str) -> bool {
        let before = self.devices.len();
        self.devices.retain(|d| d.id != device_id);
        if self.devices.len() < before {
            hotplug::emit_remove(device_id, "usb");
            true
        } else {
            false
        }
    }

    pub fn open(&mut self, device_id: &str) -> Result<u64, String> {
        let desc = self
            .devices
            .iter()
            .find(|d| d.id == device_id)
            .ok_or_else(|| format!("device not found: {device_id}"))?
            .clone();
        if desc.status != "online" {
            return Err(format!("device offline: {device_id}"));
        }
        let id = self.next_handle.fetch_add(1, Ordering::SeqCst);
        let sub_handle = match desc.kind {
            DriverKind::Usb => Some(self.usb.open(&desc.id)?),
            DriverKind::Audio => Some(self.audio.open(&desc.id, None, None)?),
            _ => None,
        };
        self.handles.insert(
            id,
            DevHandle {
                id,
                device_id: desc.id,
                kind: desc.kind,
                sub_handle,
            },
        );
        Ok(id)
    }

    pub fn close(&mut self, handle: u64) -> Result<(), String> {
        let h = self
            .handles
            .remove(&handle)
            .ok_or_else(|| format!("invalid device handle: {handle}"))?;
        if let Some(sub) = h.sub_handle {
            match h.kind {
                DriverKind::Usb => {
                    let _ = self.usb.close(sub);
                }
                DriverKind::Audio => {
                    let _ = self.audio.close(sub);
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn handle_info(&self, handle: u64) -> Result<&DevHandle, String> {
        self.handles
            .get(&handle)
            .ok_or_else(|| format!("invalid device handle: {handle}"))
    }

    pub fn ioctl(
        &mut self,
        handle: u64,
        op: &str,
        args: &[crate::value::Value],
    ) -> Result<crate::value::Value, String> {
        use crate::value::Value;
        let h = self.handle_info(handle)?.clone();
        let op = op.trim().to_ascii_lowercase();
        match h.kind {
            DriverKind::Gpu => match op.as_str() {
                "info" => Ok(gpu_info_value(&self.gpu.info())),
                "set_mode" => {
                    let w = value_u32(args, 0, "gpu set_mode width")?;
                    let h = value_u32(args, 1, "gpu set_mode height")?;
                    self.gpu.set_mode(w, h)?;
                    Ok(Value::Null)
                }
                "present" => {
                    let bytes = value_usize(args, 0, "gpu present bytes")?;
                    let tex = self.gpu.present_bytes(bytes)?;
                    Ok(Value::Number(tex as i64))
                }
                _ => Err(format!("unknown gpu ioctl: {op}")),
            },
            DriverKind::Network => match op.as_str() {
                "interfaces" => Ok(net_ifaces_value(self.net.interfaces())),
                "connect" => {
                    let host = value_string(args, 0, "net connect host")?;
                    let port = value_u16(args, 1, "net connect port")?;
                    let sock = self.net.connect(&host, port)?;
                    Ok(Value::Number(sock as i64))
                }
                "send" => {
                    let sock = value_u64(args, 0, "net send socket")?;
                    let data = value_string(args, 1, "net send data")?;
                    let n = self.net.send(sock, data.as_bytes())?;
                    Ok(Value::Number(n as i64))
                }
                "recv" => {
                    let sock = value_u64(args, 0, "net recv socket")?;
                    let max = value_usize(args, 1, "net recv max").unwrap_or(4096);
                    let buf = self.net.recv(sock, max)?;
                    Ok(Value::String(String::from_utf8_lossy(&buf).into()))
                }
                "close" => {
                    let sock = value_u64(args, 0, "net close socket")?;
                    self.net.close(sock)?;
                    Ok(Value::Null)
                }
                "listen" => {
                    let host = args
                        .first()
                        .and_then(|v| match v {
                            Value::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .unwrap_or_else(|| "0.0.0.0".into());
                    let port = value_u16(args, 1, "net listen port")?;
                    let sock = self.net.listen(&host, port)?;
                    Ok(Value::Number(sock as i64))
                }
                "accept" => {
                    let sock = value_u64(args, 0, "net accept listener")?;
                    let client = self.net.accept(sock)?;
                    Ok(Value::Number(client as i64))
                }
                "poll" => {
                    let socks: Vec<u64> = match args.first() {
                        Some(Value::Array(vals)) => vals
                            .iter()
                            .filter_map(|v| match v {
                                Value::Number(n) if *n >= 0 => Some(*n as u64),
                                _ => None,
                            })
                            .collect(),
                        Some(Value::Number(n)) if *n >= 0 => vec![*n as u64],
                        _ => Vec::new(),
                    };
                    let sock_copy = socks.clone();
                    let events = self.net.poll(&sock_copy);
                    Ok(Value::Array(
                        events
                            .into_iter()
                            .map(|e| {
                                let mut m = std::collections::HashMap::new();
                                m.insert("socket".into(), Value::Number(e.socket as i64));
                                m.insert("kind".into(), Value::String(e.kind));
                                Value::Object(m)
                            })
                            .collect(),
                    ))
                }
                "udp_bind" => {
                    let host = args
                        .first()
                        .and_then(|v| match v {
                            Value::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .unwrap_or_else(|| "0.0.0.0".into());
                    let port = value_u16(args, 1, "udp bind port")?;
                    let sock = self.net.udp_bind(&host, port)?;
                    Ok(Value::Number(sock as i64))
                }
                "udp_send" => {
                    let sock = value_u64(args, 0, "udp send socket")?;
                    let host = value_string(args, 1, "udp send host")?;
                    let port = value_u16(args, 2, "udp send port")?;
                    let data = match args.get(3) {
                        Some(Value::String(s)) => s.as_bytes().to_vec(),
                        Some(Value::Array(vals)) => vals
                            .iter()
                            .map(|v| match v {
                                Value::Number(n) => Ok(*n as u8),
                                _ => Err("udp send expects byte array".to_string()),
                            })
                            .collect::<Result<Vec<_>, _>>()?,
                        _ => Vec::new(),
                    };
                    let n = self.net.udp_send(sock, &host, port, &data)?;
                    Ok(Value::Number(n as i64))
                }
                "udp_recv" => {
                    let sock = value_u64(args, 0, "udp recv socket")?;
                    let max = value_usize(args, 1, "udp recv max").unwrap_or(4096);
                    let (buf, peer) = self.net.udp_recv(sock, max)?;
                    let mut m = std::collections::HashMap::new();
                    m.insert(
                        "data".into(),
                        Value::Array(buf.into_iter().map(|b| Value::Number(b as i64)).collect()),
                    );
                    m.insert("peer".into(), Value::String(peer));
                    Ok(Value::Object(m))
                }
                _ => Err(format!("unknown net ioctl: {op}")),
            },
            DriverKind::Usb => match op.as_str() {
                "transfer" => {
                    let sub = h.sub_handle.ok_or("usb device not opened")?;
                    let ep = args
                        .first()
                        .and_then(|v| match v {
                            Value::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .unwrap_or_else(|| "in".into());
                    let data = match args.get(1) {
                        Some(Value::String(s)) => s.as_bytes().to_vec(),
                        Some(Value::Array(vals)) => vals
                            .iter()
                            .map(|v| match v {
                                Value::Number(n) => Ok(*n as u8),
                                _ => Err("usb transfer data expects byte array".to_string()),
                            })
                            .collect::<Result<Vec<_>, _>>()?,
                        _ => Vec::new(),
                    };
                    let out = self.usb.transfer(sub, &ep, &data)?;
                    if h.device_id == "usb-serial-0" {
                        if ep == "out" && !data.is_empty() {
                            let _ = self.host.usb_serial_write(&data);
                        }
                        if (ep == "in" || ep.is_empty()) && self.host.is_enabled() {
                            if let Ok(host_bytes) = self.host.usb_serial_read(64) {
                                if !host_bytes.is_empty() {
                                    return Ok(Value::Array(
                                        host_bytes
                                            .into_iter()
                                            .map(|b| Value::Number(b as i64))
                                            .collect(),
                                    ));
                                }
                            }
                        }
                    }
                    Ok(Value::Array(
                        out.into_iter().map(|b| Value::Number(b as i64)).collect(),
                    ))
                }
                _ => Err(format!("unknown usb ioctl: {op}")),
            },
            DriverKind::Audio => {
                let sub = h.sub_handle.ok_or("audio device not opened")?;
                match op.as_str() {
                "write" => {
                    let samples = value_i16_array(args, 0, "audio write samples")?;
                    let n = self.audio.write_pcm(sub, &samples)?;
                    let _ = self.host.audio_write(&samples);
                    Ok(Value::Number(n as i64))
                }
                "read" => {
                    let frames = value_usize(args, 0, "audio read frames").unwrap_or(256);
                    let pcm = self.audio.read_pcm(sub, frames)?;
                    Ok(Value::Array(
                        pcm.into_iter().map(|s| Value::Number(s as i64)).collect(),
                    ))
                }
                "volume" => {
                    let vol = value_f32(args, 0, "audio volume")?;
                    self.audio.set_volume(sub, vol)?;
                    Ok(Value::Null)
                }
                _ => Err(format!("unknown audio ioctl: {op}")),
            }
            }
        }
    }
}

pub fn gpu_info_value(info: &GpuDriverInfo) -> crate::value::Value {
    use crate::value::Value;
    let mut m = HashMap::new();
    m.insert("device".into(), Value::String(info.device.clone()));
    m.insert("backend".into(), Value::String(info.backend.clone()));
    m.insert("available".into(), Value::Bool(info.available));
    m.insert("vram_mb".into(), Value::Number(info.vram_mb as i64));
    m.insert("width".into(), Value::Number(info.mode.width as i64));
    m.insert("height".into(), Value::Number(info.mode.height as i64));
    m.insert("bpp".into(), Value::Number(info.mode.bpp as i64));
    m.insert("present_count".into(), Value::Number(info.present_count as i64));
    if let Some(tex) = info.last_texture {
        m.insert("last_texture".into(), Value::Number(tex as i64));
    }
    Value::Object(m)
}

pub fn net_ifaces_value(ifaces: &[NetInterface]) -> crate::value::Value {
    use crate::value::Value;
    Value::Array(
        ifaces
            .iter()
            .map(|i| {
                let mut m = HashMap::new();
                m.insert("name".into(), Value::String(i.name.clone()));
                m.insert("mac".into(), Value::String(i.mac.clone()));
                m.insert("ipv4".into(), Value::String(i.ipv4.clone()));
                m.insert("up".into(), Value::Bool(i.up));
                m.insert("mtu".into(), Value::Number(i.mtu as i64));
                Value::Object(m)
            })
            .collect(),
    )
}

pub fn device_list_value(devices: &[DeviceDescriptor]) -> crate::value::Value {
    use crate::value::Value;
    Value::Array(
        devices
            .iter()
            .map(|d| {
                let mut m = HashMap::new();
                m.insert("id".into(), Value::String(d.id.clone()));
                m.insert("kind".into(), Value::String(d.kind.as_str().into()));
                m.insert("name".into(), Value::String(d.name.clone()));
                m.insert("vendor".into(), Value::String(d.vendor.clone()));
                m.insert("status".into(), Value::String(d.status.into()));
                Value::Object(m)
            })
            .collect(),
    )
}

fn value_string(args: &[crate::value::Value], i: usize, name: &str) -> Result<String, String> {
    match args.get(i) {
        Some(crate::value::Value::String(s)) => Ok(s.clone()),
        _ => Err(format!("{name} expects string")),
    }
}

fn value_u32(args: &[crate::value::Value], i: usize, name: &str) -> Result<u32, String> {
    match args.get(i) {
        Some(crate::value::Value::Number(n)) if *n >= 0 => Ok(*n as u32),
        _ => Err(format!("{name} expects number")),
    }
}

fn value_u16(args: &[crate::value::Value], i: usize, name: &str) -> Result<u16, String> {
    Ok(value_u32(args, i, name)? as u16)
}

fn value_u64(args: &[crate::value::Value], i: usize, name: &str) -> Result<u64, String> {
    match args.get(i) {
        Some(crate::value::Value::Number(n)) if *n >= 0 => Ok(*n as u64),
        _ => Err(format!("{name} expects number")),
    }
}

fn value_usize(args: &[crate::value::Value], i: usize, name: &str) -> Result<usize, String> {
    Ok(value_u64(args, i, name)? as usize)
}

fn value_f32(args: &[crate::value::Value], i: usize, name: &str) -> Result<f32, String> {
    match args.get(i) {
        Some(crate::value::Value::Float(f)) => Ok(*f as f32),
        Some(crate::value::Value::Number(n)) => Ok(*n as f32),
        _ => Err(format!("{name} expects number")),
    }
}

fn value_i16_array(args: &[crate::value::Value], i: usize, name: &str) -> Result<Vec<i16>, String> {
    match args.get(i) {
        Some(crate::value::Value::Array(vals)) => vals
            .iter()
            .map(|v| match v {
                crate::value::Value::Number(n) => Ok(*n as i16),
                _ => Err(format!("{name} expects array of numbers")),
            })
            .collect(),
        _ => Err(format!("{name} expects array")),
    }
}
