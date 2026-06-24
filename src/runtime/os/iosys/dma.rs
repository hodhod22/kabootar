//! DMA manager — direct memory access channels.

#[derive(Debug, Clone)]
pub struct DmaChannel {
    pub id: u64,
    pub device: String,
    pub buffer_size: usize,
    pub active: bool,
}

impl DmaChannel {
    pub fn new(id: u64, device: &str, size: usize) -> Self {
        Self {
            id,
            device: device.to_string(),
            buffer_size: size,
            active: true,
        }
    }

    pub fn transfer(&self, _src: u64, _dst: u64, len: usize) -> Result<usize, String> {
        if len > self.buffer_size {
            return Err("dma transfer exceeds buffer".into());
        }
        Ok(len)
    }
}
