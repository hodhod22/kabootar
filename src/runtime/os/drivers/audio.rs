//! Kabootar OS audio driver — PCM output/input (virtual + cpal host devices).

use crate::runtime::os::native_hw::{self, AudioBackend};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioDirection {
    Output,
    Input,
}

impl AudioDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            AudioDirection::Output => "output",
            AudioDirection::Input => "input",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub direction: AudioDirection,
    pub channels: u8,
    pub sample_rate: u32,
}

#[derive(Debug)]
struct OpenAudio {
    device_id: String,
    direction: AudioDirection,
    channels: u8,
    sample_rate: u32,
    buffer: Vec<i16>,
    samples_written: u64,
    volume: f32,
    native_handle: Option<u64>,
}

pub struct AudioDriver {
    devices: Vec<AudioDeviceInfo>,
    open: HashMap<u64, OpenAudio>,
    next_handle: AtomicU64,
    native: AudioBackend,
}

impl Default for AudioDriver {
    fn default() -> Self {
        let mut driver = Self {
            devices: vec![
                AudioDeviceInfo {
                    id: "audio-out-0".into(),
                    name: "Kabootar PCM Out (virtual)".into(),
                    direction: AudioDirection::Output,
                    channels: 2,
                    sample_rate: 44100,
                },
                AudioDeviceInfo {
                    id: "audio-in-0".into(),
                    name: "Kabootar PCM In (virtual)".into(),
                    direction: AudioDirection::Input,
                    channels: 1,
                    sample_rate: 16000,
                },
            ],
            open: HashMap::new(),
            next_handle: AtomicU64::new(1),
            native: AudioBackend::default(),
        };
        driver.refresh_host();
        driver
    }
}

impl AudioDriver {
    pub fn list(&self) -> &[AudioDeviceInfo] {
        &self.devices
    }

    pub fn refresh_host(&mut self) {
        if !native_hw::enabled() {
            return;
        }
        self.native.rescan();
        self.devices
            .retain(|d| !d.id.starts_with("host-audio-"));
        self.devices.extend(self.native.device_infos());
    }

    pub fn native_available(&self) -> bool {
        self.native.is_available()
    }

    pub fn merge_host_devices(&mut self, host: &[AudioDeviceInfo]) {
        self.devices
            .retain(|d| !d.id.starts_with("host-audio-"));
        self.devices.extend(host.iter().cloned());
    }

    pub fn open(
        &mut self,
        device_id: &str,
        channels: Option<u8>,
        sample_rate: Option<u32>,
    ) -> Result<u64, String> {
        let dev = self
            .devices
            .iter()
            .find(|d| d.id == device_id)
            .ok_or_else(|| format!("audio device not found: {device_id}"))?;
        let ch = channels.unwrap_or(dev.channels).clamp(1, 8);
        let rate = sample_rate.unwrap_or(dev.sample_rate).clamp(8000, 192000);
        let id = self.next_handle.fetch_add(1, Ordering::SeqCst);
        let native_handle = if self.native.is_host_device(device_id) {
            Some(self.native.open(device_id, Some(ch), Some(rate))?)
        } else {
            None
        };
        self.open.insert(
            id,
            OpenAudio {
                device_id: dev.id.clone(),
                direction: dev.direction,
                channels: ch,
                sample_rate: rate,
                buffer: Vec::new(),
                samples_written: 0,
                volume: 1.0,
                native_handle,
            },
        );
        Ok(id)
    }

    pub fn close(&mut self, handle: u64) -> Result<(), String> {
        let entry = self
            .open
            .remove(&handle)
            .ok_or_else(|| format!("invalid audio handle: {handle}"))?;
        if let Some(nh) = entry.native_handle {
            self.native.close(nh);
        }
        Ok(())
    }

    pub fn write_pcm(&mut self, handle: u64, samples: &[i16]) -> Result<usize, String> {
        let entry = self
            .open
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid audio handle: {handle}"))?;
        if entry.direction != AudioDirection::Output {
            return Err("device is not an output".into());
        }
        if samples.is_empty() {
            return Err("audio write: empty buffer".into());
        }
        let vol = entry.volume.clamp(0.0, 2.0);
        let scaled: Vec<i16> = samples
            .iter()
            .map(|&s| (s as f32 * vol).clamp(i16::MIN as f32, i16::MAX as f32) as i16)
            .collect();
        if let Some(nh) = entry.native_handle {
            let n = self.native.write_pcm(nh, &scaled)?;
            entry.samples_written += n as u64;
            return Ok(n);
        }
        entry.buffer.extend_from_slice(&scaled);
        entry.samples_written += scaled.len() as u64;
        Ok(scaled.len())
    }

    pub fn read_pcm(&mut self, handle: u64, frames: usize) -> Result<Vec<i16>, String> {
        let entry = self
            .open
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid audio handle: {handle}"))?;
        if entry.direction != AudioDirection::Input {
            return Err("device is not an input".into());
        }
        if let Some(nh) = entry.native_handle {
            let out = self.native.read_pcm(nh, frames)?;
            entry.samples_written += out.len() as u64;
            return Ok(out);
        }
        let ch = entry.channels as usize;
        let n = frames.saturating_mul(ch).min(8192);
        let mut out = Vec::with_capacity(n);
        let base = entry.samples_written as f32;
        for i in 0..n {
            let t = (base + i as f32) / entry.sample_rate as f32;
            let v = (t * 440.0 * std::f32::consts::TAU).sin() * 800.0;
            out.push(v as i16);
        }
        entry.samples_written += n as u64;
        Ok(out)
    }

    pub fn set_volume(&mut self, handle: u64, volume: f32) -> Result<(), String> {
        let entry = self
            .open
            .get_mut(&handle)
            .ok_or_else(|| format!("invalid audio handle: {handle}"))?;
        entry.volume = volume.clamp(0.0, 2.0);
        Ok(())
    }

    pub fn stats(&self, handle: u64) -> Result<(u64, usize, u32, u8), String> {
        let entry = self
            .open
            .get(&handle)
            .ok_or_else(|| format!("invalid audio handle: {handle}"))?;
        Ok((
            entry.samples_written,
            entry.buffer.len(),
            entry.sample_rate,
            entry.channels,
        ))
    }
}
