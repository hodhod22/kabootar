//! Native host audio via cpal (WASAPI / ALSA / CoreAudio).

use super::super::drivers::audio::{AudioDeviceInfo, AudioDirection};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct HostAudioDesc {
    pub id: String,
    pub name: String,
    pub direction: AudioDirection,
    pub sample_rate: u32,
    pub channels: u8,
    cpal_name: String,
    is_output: bool,
}

struct OutputStream {
    queue: Arc<Mutex<VecDeque<i16>>>,
    _stream: cpal::Stream,
}

struct InputStream {
    ring: Arc<Mutex<VecDeque<i16>>>,
    _stream: cpal::Stream,
}

pub struct AudioBackend {
    devices: Vec<HostAudioDesc>,
    outputs: HashMap<u64, OutputStream>,
    inputs: HashMap<u64, InputStream>,
    next: AtomicU64,
    available: bool,
    error: Option<String>,
}

impl Default for AudioBackend {
    fn default() -> Self {
        let mut backend = Self {
            devices: Vec::new(),
            outputs: HashMap::new(),
            inputs: HashMap::new(),
            next: AtomicU64::new(1),
            available: false,
            error: None,
        };
        backend.rescan();
        backend
    }
}

impl AudioBackend {
    pub fn is_available(&self) -> bool {
        self.available
    }

    pub fn last_error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn rescan(&mut self) {
        self.devices.clear();
        match cpal::default_host() {
            host => {
                let mut ok = false;
                if let Ok(devs) = host.output_devices() {
                    for (i, dev) in devs.enumerate() {
                        if let Ok(name) = dev.name() {
                            let cfg = dev.default_output_config().ok();
                            let rate = cfg.as_ref().map(|c| c.sample_rate().0).unwrap_or(44100);
                            let ch = cfg.map(|c| c.channels()).unwrap_or(2).min(8) as u8;
                            self.devices.push(HostAudioDesc {
                                id: format!("host-audio-out-{i}"),
                                name: name.clone(),
                                direction: AudioDirection::Output,
                                sample_rate: rate,
                                channels: ch,
                                cpal_name: name,
                                is_output: true,
                            });
                            ok = true;
                        }
                    }
                }
                if let Ok(devs) = host.input_devices() {
                    for (i, dev) in devs.enumerate() {
                        if let Ok(name) = dev.name() {
                            let cfg = dev.default_input_config().ok();
                            let rate = cfg.as_ref().map(|c| c.sample_rate().0).unwrap_or(44100);
                            let ch = cfg.map(|c| c.channels()).unwrap_or(1).min(8) as u8;
                            self.devices.push(HostAudioDesc {
                                id: format!("host-audio-in-{i}"),
                                name: name.clone(),
                                direction: AudioDirection::Input,
                                sample_rate: rate,
                                channels: ch,
                                cpal_name: name,
                                is_output: false,
                            });
                            ok = true;
                        }
                    }
                }
                self.available = ok;
                if !ok {
                    self.error = Some("no cpal audio devices found".into());
                } else {
                    self.error = None;
                }
            }
        }
    }

    pub fn device_infos(&self) -> Vec<AudioDeviceInfo> {
        self.devices
            .iter()
            .map(|d| AudioDeviceInfo {
                id: d.id.clone(),
                name: d.name.clone(),
                direction: d.direction,
                channels: d.channels,
                sample_rate: d.sample_rate,
            })
            .collect()
    }

    pub fn is_host_device(&self, device_id: &str) -> bool {
        device_id.starts_with("host-audio-")
    }

    pub fn open(
        &mut self,
        device_id: &str,
        channels: Option<u8>,
        sample_rate: Option<u32>,
    ) -> Result<u64, String> {
        let desc = self
            .devices
            .iter()
            .find(|d| d.id == device_id)
            .ok_or_else(|| format!("host audio device not found: {device_id}"))?
            .clone();
        let ch = channels.unwrap_or(desc.channels).clamp(1, 8);
        let rate = sample_rate.unwrap_or(desc.sample_rate).clamp(8000, 192000);
        let host = cpal::default_host();
        let device = find_device(&host, &desc.cpal_name, desc.is_output)
            .ok_or_else(|| format!("cpal device gone: {}", desc.cpal_name))?;
        let id = self.next.fetch_add(1, Ordering::SeqCst);
        if desc.is_output {
            let stream = build_output(&device, ch, rate)?;
            self.outputs.insert(id, stream);
        } else {
            let stream = build_input(&device, ch, rate)?;
            self.inputs.insert(id, stream);
        }
        Ok(id)
    }

    pub fn close(&mut self, handle: u64) -> bool {
        self.outputs.remove(&handle).is_some() || self.inputs.remove(&handle).is_some()
    }

    pub fn write_pcm(&mut self, handle: u64, samples: &[i16]) -> Result<usize, String> {
        let stream = self
            .outputs
            .get(&handle)
            .ok_or_else(|| format!("invalid host audio output handle: {handle}"))?;
        let mut q = stream
            .queue
            .lock()
            .map_err(|_| "audio queue lock poisoned".to_string())?;
        for &s in samples {
            q.push_back(s);
        }
        Ok(samples.len())
    }

    pub fn read_pcm(&mut self, handle: u64, frames: usize) -> Result<Vec<i16>, String> {
        let stream = self
            .inputs
            .get(&handle)
            .ok_or_else(|| format!("invalid host audio input handle: {handle}"))?;
        let mut ring = stream
            .ring
            .lock()
            .map_err(|_| "audio ring lock poisoned".to_string())?;
        let want = frames.min(8192);
        let mut out = Vec::with_capacity(want);
        for _ in 0..want {
            match ring.pop_front() {
                Some(s) => out.push(s),
                None => break,
            }
        }
        Ok(out)
    }
}

fn find_device(host: &cpal::Host, name: &str, output: bool) -> Option<cpal::Device> {
    let mut iter = if output {
        host.output_devices().ok()?
    } else {
        host.input_devices().ok()?
    };
    iter.find(|d| d.name().ok().as_deref() == Some(name))
}

fn build_output(device: &cpal::Device, channels: u8, rate: u32) -> Result<OutputStream, String> {
    let queue = Arc::new(Mutex::new(VecDeque::new()));
    let q = Arc::clone(&queue);
    let config = cpal::StreamConfig {
        channels: channels as u16,
        sample_rate: cpal::SampleRate(rate),
        buffer_size: cpal::BufferSize::Default,
    };
    let stream = device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                if let Ok(mut guard) = q.lock() {
                    for sample in data.iter_mut() {
                        *sample = guard
                            .pop_front()
                            .map(|s| s as f32 / i16::MAX as f32)
                            .unwrap_or(0.0);
                    }
                }
            },
            |e| eprintln!("kabootar cpal output: {e}"),
            None,
        )
        .map_err(|e| format!("cpal output stream: {e}"))?;
    stream.play().map_err(|e| format!("cpal play: {e}"))?;
    Ok(OutputStream {
        queue,
        _stream: stream,
    })
}

fn build_input(device: &cpal::Device, channels: u8, rate: u32) -> Result<InputStream, String> {
    let ring = Arc::new(Mutex::new(VecDeque::with_capacity(8192)));
    let r = Arc::clone(&ring);
    let config = cpal::StreamConfig {
        channels: channels as u16,
        sample_rate: cpal::SampleRate(rate),
        buffer_size: cpal::BufferSize::Default,
    };
    let stream = device
        .build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if let Ok(mut guard) = r.lock() {
                    for &sample in data {
                        let s = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                        guard.push_back(s);
                        if guard.len() > 16384 {
                            guard.pop_front();
                        }
                    }
                }
            },
            |e| eprintln!("kabootar cpal input: {e}"),
            None,
        )
        .map_err(|e| format!("cpal input stream: {e}"))?;
    stream.play().map_err(|e| format!("cpal input play: {e}"))?;
    Ok(InputStream {
        ring,
        _stream: stream,
    })
}
