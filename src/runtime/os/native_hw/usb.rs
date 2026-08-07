//! Full native USB — serialport (CDC) + hidapi (HID) + nusb (enumeration + control).

use super::super::drivers::usb::{UsbClass, UsbDeviceInfo};
use hidapi::{HidApi, HidDevice};
use nusb::transfer::{ControlIn, ControlOut, ControlType, Recipient};
use nusb::{DeviceId, MaybeFuture};
use serialport::{available_ports, SerialPort, SerialPortType};
use std::collections::HashMap;
use std::ffi::CString;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

#[derive(Clone)]
enum HostUsbKind {
    Serial { port_name: String },
    Hid { path: CString },
    Nusb {
        device_id: DeviceId,
        class: UsbClass,
    },
}

#[derive(Clone)]
struct HostUsbDesc {
    info: UsbDeviceInfo,
    kind: HostUsbKind,
}

enum OpenHandle {
    Serial(Box<dyn SerialPort>),
    Hid(HidDevice),
    NusbIf(nusb::Interface),
}

pub struct UsbBackend {
    devices: Vec<HostUsbDesc>,
    open: HashMap<u64, OpenHandle>,
    hid_api: Option<HidApi>,
    next: AtomicU64,
    available: bool,
    error: Option<String>,
    hid_count: usize,
    nusb_count: usize,
    serial_count: usize,
}

impl Default for UsbBackend {
    fn default() -> Self {
        let mut backend = Self {
            devices: Vec::new(),
            open: HashMap::new(),
            hid_api: HidApi::new().ok(),
            next: AtomicU64::new(1),
            available: false,
            error: None,
            hid_count: 0,
            nusb_count: 0,
            serial_count: 0,
        };
        backend.rescan();
        backend
    }
}

impl UsbBackend {
    pub fn is_available(&self) -> bool {
        self.available
    }

    pub fn last_error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn rescan(&mut self) {
        self.devices.clear();
        self.hid_count = 0;
        self.nusb_count = 0;
        self.serial_count = 0;
        let mut ok = false;
        let mut seen_vid_pid: Vec<(u16, u16)> = Vec::new();

        if let Some(api) = &self.hid_api {
            for (i, dev) in api.device_list().enumerate() {
                let vid = dev.vendor_id();
                let pid = dev.product_id();
                seen_vid_pid.push((vid, pid));
                let vendor = format!("{vid:04X}");
                let product = dev
                    .product_string()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("HID {pid:04X}"));
                let path = CString::from(dev.path());
                let id = format!("host-usb-hid-{i}");
                self.devices.push(HostUsbDesc {
                    info: UsbDeviceInfo {
                        id: id.clone(),
                        vendor,
                        product,
                        class: UsbClass::Hid,
                        bus: 0,
                        address: i as u8,
                    },
                    kind: HostUsbKind::Hid { path },
                });
                self.hid_count += 1;
                ok = true;
            }
        }

        if let Ok(ports) = available_ports() {
            for p in ports {
                let (vendor, product, vid, pid) = match &p.port_type {
                    SerialPortType::UsbPort(u) => (
                        format!("{:04X}", u.vid),
                        format!(
                            "{} {:04X}:{:04X}",
                            u.serial_number.as_deref().unwrap_or("USB Serial"),
                            u.vid,
                            u.pid
                        ),
                        u.vid,
                        u.pid,
                    ),
                    SerialPortType::BluetoothPort => {
                        ("Bluetooth".into(), p.port_name.clone(), 0, 0)
                    }
                    SerialPortType::PciPort => ("PCI".into(), p.port_name.clone(), 0, 0),
                    SerialPortType::Unknown => ("host".into(), p.port_name.clone(), 0, 0),
                };
                if vid != 0 {
                    seen_vid_pid.push((vid, pid));
                }
                let id = format!("host-usb-serial-{}", sanitize_id(&p.port_name));
                self.devices.push(HostUsbDesc {
                    info: UsbDeviceInfo {
                        id: id.clone(),
                        vendor,
                        product,
                        class: UsbClass::CdcAcm,
                        bus: 0,
                        address: 0,
                    },
                    kind: HostUsbKind::Serial {
                        port_name: p.port_name,
                    },
                });
                self.serial_count += 1;
                ok = true;
            }
        }

        match nusb::list_devices().wait() {
            Ok(list) => {
                for dev in list.iter() {
                    let vid = dev.vendor_id();
                    let pid = dev.product_id();
                    let bus = nusb_bus_num(&dev);
                    let addr = dev.device_address();
                    let class = UsbClass::from_usb_code(dev.class());
                    if class == UsbClass::Hid && seen_vid_pid.contains(&(vid, pid)) {
                        continue;
                    }
                    let vendor = dev
                        .manufacturer_string()
                        .unwrap_or("")
                        .to_string();
                    let product = dev
                        .product_string()
                        .unwrap_or("")
                        .to_string();
                    let id = format!("host-usb-{vid:04x}-{pid:04x}-b{bus}a{addr}");
                    self.devices.push(HostUsbDesc {
                        info: UsbDeviceInfo {
                            id,
                            vendor: if vendor.is_empty() {
                                format!("{vid:04X}")
                            } else {
                                vendor
                            },
                            product: if product.is_empty() {
                                format!("{pid:04X}")
                            } else {
                                product
                            },
                            class,
                            bus,
                            address: addr,
                        },
                        kind: HostUsbKind::Nusb {
                            device_id: dev.id(),
                            class,
                        },
                    });
                    self.nusb_count += 1;
                    ok = true;
                }
            }
            Err(e) => {
                if !ok {
                    self.error = Some(format!("nusb scan: {e}"));
                }
            }
        }

        self.available = ok;
        if ok {
            self.error = None;
        } else if self.error.is_none() {
            self.error = Some("no host USB devices found".into());
        }
    }

    pub fn device_infos(&self) -> Vec<UsbDeviceInfo> {
        self.devices.iter().map(|d| d.info.clone()).collect()
    }

    pub fn stats(&self) -> (usize, usize, usize) {
        (self.hid_count, self.serial_count, self.nusb_count)
    }

    pub fn is_host_device(&self, device_id: &str) -> bool {
        device_id.starts_with("host-usb-")
    }

    pub fn is_serial_device(&self, device_id: &str) -> bool {
        device_id.starts_with("host-usb-serial-")
    }

    pub fn is_hid_device(&self, device_id: &str) -> bool {
        device_id.starts_with("host-usb-hid-")
    }

    pub fn is_openable(&self, device_id: &str) -> bool {
        self.devices
            .iter()
            .find(|d| d.info.id == device_id)
            .is_some_and(|d| match &d.kind {
                HostUsbKind::Serial { .. } | HostUsbKind::Hid { .. } => true,
                HostUsbKind::Nusb { class, .. } => *class != UsbClass::MassStorage,
            })
    }

    pub fn open(&mut self, device_id: &str) -> Result<u64, String> {
        let desc = self
            .devices
            .iter()
            .find(|d| d.info.id == device_id)
            .ok_or_else(|| format!("host usb device not found: {device_id}"))?
            .clone();
        let handle = match desc.kind {
            HostUsbKind::Serial { port_name } => {
                let port = serialport::new(&port_name, 115_200)
                    .timeout(Duration::from_millis(100))
                    .open()
                    .map_err(|e| format!("serial open {port_name}: {e}"))?;
                OpenHandle::Serial(port)
            }
            HostUsbKind::Hid { path } => {
                let api = self
                    .hid_api
                    .as_ref()
                    .ok_or("hidapi not initialized")?;
                let dev = api
                    .open_path(path.as_c_str())
                    .map_err(|e| format!("hid open: {e}"))?;
                let _ = dev.set_blocking_mode(false);
                OpenHandle::Hid(dev)
            }
            HostUsbKind::Nusb { device_id, class } => {
                if class == UsbClass::MassStorage {
                    return Err(
                        "USB mass-storage requires OS-level mounting (enumeration only)".into(),
                    );
                }
                let iface = open_nusb_interface(device_id)?;
                OpenHandle::NusbIf(iface)
            }
        };
        let id = self.next.fetch_add(1, Ordering::SeqCst);
        self.open.insert(id, handle);
        Ok(id)
    }

    pub fn close(&mut self, handle: u64) -> bool {
        self.open.remove(&handle).is_some()
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
            .ok_or_else(|| format!("invalid host usb handle: {handle}"))?;
        let ep = endpoint.trim().to_ascii_lowercase();
        match entry {
            OpenHandle::Serial(port) => transfer_serial(port, &ep, data),
            OpenHandle::Hid(hid) => transfer_hid(hid, &ep, data),
            OpenHandle::NusbIf(iface) => transfer_nusb_iface(iface, &ep, data),
        }
    }
}

fn transfer_serial(
    port: &mut Box<dyn SerialPort>,
    ep: &str,
    data: &[u8],
) -> Result<Vec<u8>, String> {
    if ep == "out" {
        let n = port.write(data).map_err(|e| format!("serial write: {e}"))?;
        port.flush().ok();
        return Ok(vec![n as u8]);
    }
    let mut buf = vec![0u8; 4096];
    match port.read(&mut buf) {
        Ok(0) => Ok(Vec::new()),
        Ok(n) => {
            buf.truncate(n);
            Ok(buf)
        }
        Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Ok(Vec::new()),
        Err(e) => Err(format!("serial read: {e}")),
    }
}

fn transfer_hid(hid: &HidDevice, ep: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    if ep == "out" {
        let n = hid
            .write(data)
            .map_err(|e| format!("hid write: {e}"))?;
        return Ok(vec![n as u8]);
    }
    let mut buf = vec![0u8; 64];
    match hid.read_timeout(&mut buf, 50) {
        Ok(0) => Ok(Vec::new()),
        Ok(n) => {
            buf.truncate(n);
            Ok(buf)
        }
        Err(e) => Err(format!("hid read: {e}")),
    }
}

fn transfer_nusb_iface(
    iface: &nusb::Interface,
    ep: &str,
    data: &[u8],
) -> Result<Vec<u8>, String> {
    if ep != "control" && ep != "in" && ep != "out" {
        return Err(format!(
            "nusb interface supports endpoint 'control' (got '{ep}')"
        ));
    }
    if data.len() < 6 {
        return Err(
            "control transfer needs [bmRequestType, bRequest, wValue_lo, wValue_hi, wIndex_lo, wIndex_hi, ...]"
                .into(),
        );
    }
    let bm = data[0];
    let request = data[1];
    let value = u16::from_le_bytes([data[2], data[3]]);
    let index = u16::from_le_bytes([data[4], data[5]]);
    let payload = &data[6..];
    let (control_type, recipient) = decode_bm(bm);
    let timeout = Duration::from_millis(500);

    if bm & 0x80 != 0 {
        let length = payload.first().copied().unwrap_or(64) as u16;
        let ci = ControlIn {
            control_type,
            recipient,
            request,
            value,
            index,
            length,
        };
        let resp = iface
            .control_in(ci, timeout)
            .wait()
            .map_err(|e| format!("nusb control_in: {e}"))?;
        return Ok(resp);
    }

    let co = ControlOut {
        control_type,
        recipient,
        request,
        value,
        index,
        data: payload,
    };
    iface
        .control_out(co, timeout)
        .wait()
        .map_err(|e| format!("nusb control_out: {e}"))?;
    Ok(vec![payload.len() as u8])
}

fn decode_bm(bm: u8) -> (ControlType, Recipient) {
    let control_type = match bm & 0x60 {
        0x00 => ControlType::Standard,
        0x20 => ControlType::Class,
        _ => ControlType::Vendor,
    };
    let recipient = match bm & 0x1f {
        0x00 => Recipient::Device,
        0x01 => Recipient::Interface,
        0x02 => Recipient::Endpoint,
        _ => Recipient::Device,
    };
    (control_type, recipient)
}

fn open_nusb_interface(device_id: DeviceId) -> Result<nusb::Interface, String> {
    let mut list = nusb::list_devices()
        .wait()
        .map_err(|e| format!("list_devices: {e}"))?;
    let info = list
        .find(|d| d.id() == device_id)
        .ok_or_else(|| "nusb device no longer connected".to_string())?;
    let device = info
        .open()
        .wait()
        .map_err(|e| format!("device open: {e}"))?;
    let iface_num = info
        .interfaces()
        .next()
        .map(|i| i.interface_number())
        .unwrap_or(0);
    device
        .claim_interface(iface_num)
        .wait()
        .map_err(|e| format!("claim_interface({iface_num}): {e}"))
}

fn sanitize_id(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn nusb_bus_num(dev: &nusb::DeviceInfo) -> u8 {
    #[cfg(target_os = "linux")]
    {
        dev.busnum()
    }
    #[cfg(not(target_os = "linux"))]
    {
        dev.bus_id()
            .chars()
            .filter(|c| c.is_ascii_digit())
            .take(3)
            .collect::<String>()
            .parse::<u8>()
            .unwrap_or(0)
    }
}
