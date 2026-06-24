//! Kabootar OS USB driver — virtual + host serial (serialport) + nusb enumeration.

use crate::runtime::os::native_hw::{self, UsbBackend};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbClass {
    Hid,
    MassStorage,
    CdcAcm,
    /// Generic USB device (enumeration / control transfers via nusb).
    Device,
}

impl UsbClass {
    pub fn as_str(self) -> &'static str {
        match self {
            UsbClass::Hid => "hid",
            UsbClass::MassStorage => "mass-storage",
            UsbClass::CdcAcm => "cdc-acm",
            UsbClass::Device => "device",
        }
    }

    pub fn from_usb_code(code: u8) -> Self {
        match code {
            0x03 => UsbClass::Hid,
            0x08 => UsbClass::MassStorage,
            0x02 | 0x0A => UsbClass::CdcAcm,
            _ => UsbClass::Device,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UsbDeviceInfo {
    pub id: String,
    pub vendor: String,
    pub product: String,
    pub class: UsbClass,
    pub bus: u8,
    pub address: u8,
}

#[derive(Debug)]
struct OpenUsb {
    device_id: String,
    class: UsbClass,
    sectors: HashMap<u32, Vec<u8>>,
    hid_keys: Vec<u8>,
    serial_rx: Vec<u8>,
    native_handle: Option<u64>,
}

pub struct UsbDriver {
    devices: Vec<UsbDeviceInfo>,
    open: HashMap<u64, OpenUsb>,
    next_handle: AtomicU64,
    transfers: u64,
    native: UsbBackend,
}

impl Default for UsbDriver {
    fn default() -> Self {
        let mut driver = Self {
            devices: vec![
                UsbDeviceInfo {
                    id: "usb-hid-0".into(),
                    vendor: "Kabootar".into(),
                    product: "Virtual Keyboard".into(),
                    class: UsbClass::Hid,
                    bus: 1,
                    address: 2,
                },
                UsbDeviceInfo {
                    id: "usb-ms-0".into(),
                    vendor: "Kabootar".into(),
                    product: "Virtual Flash Drive".into(),
                    class: UsbClass::MassStorage,
                    bus: 1,
                    address: 3,
                },
                UsbDeviceInfo {
                    id: "usb-serial-0".into(),
                    vendor: "Kabootar".into(),
                    product: "Virtual Serial".into(),
                    class: UsbClass::CdcAcm,
                    bus: 1,
                    address: 4,
                },
            ],
            open: HashMap::new(),
            next_handle: AtomicU64::new(1),
            transfers: 0,
            native: UsbBackend::default(),
        };
        driver.refresh_host();
        driver
    }
}

impl UsbDriver {
    pub fn list(&self) -> &[UsbDeviceInfo] {
        &self.devices
    }

    pub fn refresh_host(&mut self) {
        if !native_hw::enabled() {
            return;
        }
        self.native.rescan();
        self.devices
            .retain(|d| !d.id.starts_with("host-usb-"));
        self.devices.extend(self.native.device_infos());
    }

    pub fn native_available(&self) -> bool {
        self.native.is_available()
    }

    pub fn native_stats(&self) -> (usize, usize, usize) {
        self.native.stats()
    }

    pub fn open(&mut self, device_id: &str) -> Result<u64, String> {
        let dev = self
            .devices
            .iter()
            .find(|d| d.id == device_id)
            .ok_or_else(|| format!("usb device not found: {device_id}"))?;
        let id = self.next_handle.fetch_add(1, Ordering::SeqCst);
        let mut sectors = HashMap::new();
        if dev.class == UsbClass::MassStorage {
            sectors.insert(0, vec![0u8; 512]);
        }
        let native_handle = if self.native.is_host_device(device_id) {
            if self.native.is_openable(device_id) {
                Some(self.native.open(device_id)?)
            } else {
                None
            }
        } else {
            None
        };
        self.open.insert(
            id,
            OpenUsb {
                device_id: dev.id.clone(),
                class: dev.class,
                sectors,
                hid_keys: vec![0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00],
                serial_rx: b"Kabootar USB serial ready\n".to_vec(),
                native_handle,
            },
        );
        Ok(id)
    }

    pub fn close(&mut self, handle: u64) -> Result<(), String> {
        let entry = self
            .open
            .remove(&handle)
            .ok_or_else(|| format!("invalid usb handle: {handle}"))?;
        if let Some(nh) = entry.native_handle {
            self.native.close(nh);
        }
        Ok(())
    }

    pub fn transfer(
        &mut self,
        handle: u64,
        endpoint: &str,
        data: &[u8],
    ) -> Result<Vec<u8>, String> {
        let entry = self
            .open
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid usb handle: {handle}"))?;
        self.transfers += 1;
        if let Some(nh) = entry.native_handle {
            return self.native.transfer(nh, endpoint, data);
        }
        let ep = endpoint.trim().to_ascii_lowercase();

        match entry.class {
            UsbClass::Hid => {
                if ep == "in" || ep.is_empty() {
                    Ok(entry.hid_keys.clone())
                } else if ep == "out" {
                    if !data.is_empty() {
                        entry.hid_keys = data.to_vec();
                    }
                    Ok(vec![data.len() as u8])
                } else {
                    Err(format!("unknown hid endpoint: {endpoint}"))
                }
            }
            UsbClass::MassStorage => {
                if data.len() < 2 {
                    return Err("mass-storage transfer needs at least 2 bytes".into());
                }
                let cmd = data[0];
                let sector = u32::from_le_bytes([data.get(1).copied().unwrap_or(0), 0, 0, 0]);
                match cmd {
                    0x01 => {
                        let sec = entry
                            .sectors
                            .entry(sector)
                            .or_insert_with(|| vec![0u8; 512]);
                        Ok(sec.clone())
                    }
                    0x02 => {
                        let payload = if data.len() > 6 { &data[6..] } else { &[] };
                        let sec = entry.sectors.entry(sector).or_insert_with(|| vec![0u8; 512]);
                        let n = payload.len().min(512);
                        sec[..n].copy_from_slice(&payload[..n]);
                        Ok(vec![n as u8])
                    }
                    _ => Err(format!("unknown mass-storage command: {cmd}")),
                }
            }
            UsbClass::CdcAcm => {
                if ep == "in" || ep.is_empty() {
                    let n = entry.serial_rx.len().min(64);
                    let out = entry.serial_rx.drain(..n).collect();
                    Ok(out)
                } else if ep == "out" {
                    Ok(vec![data.len() as u8])
                } else {
                    Err(format!("unknown serial endpoint: {endpoint}"))
                }
            }
            UsbClass::Device => Err(format!(
                "device {} requires host open for control transfers",
                entry.device_id
            )),
        }
    }

    pub fn hotplug(&mut self, vendor: &str, product: &str, class: UsbClass) -> String {
        let id = format!("usb-{}-{}", class.as_str(), self.devices.len());
        let address = (self.devices.len() + 5) as u8;
        self.devices.push(UsbDeviceInfo {
            id: id.clone(),
            vendor: vendor.into(),
            product: product.into(),
            class,
            bus: 1,
            address,
        });
        id
    }

    pub fn transfer_count(&self) -> u64 {
        self.transfers
    }
}
