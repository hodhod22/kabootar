//! 8 KB page format for disk-backed tables (Phase 2).

pub const PAGE_SIZE: usize = 8192;
pub const PAGE_MAGIC: u32 = 0x4B_44_42_50; // KDBP

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PageKind {
    Free = 0,
    Heap = 1,
    Index = 2,
    Meta = 3,
}

#[derive(Debug, Clone)]
pub struct Page {
    pub id: u64,
    pub kind: PageKind,
    pub data: Vec<u8>,
    pub dirty: bool,
    pub pin_count: u32,
}

impl Page {
    pub fn new(id: u64, kind: PageKind) -> Self {
        Self {
            id,
            kind,
            data: vec![0u8; PAGE_SIZE],
            dirty: false,
            pin_count: 0,
        }
    }

    pub fn encode_header(&mut self) {
        if self.data.len() < 16 {
            self.data.resize(PAGE_SIZE, 0);
        }
        self.data[0..4].copy_from_slice(&PAGE_MAGIC.to_le_bytes());
        self.data[4] = self.kind as u8;
        self.data[5..13].copy_from_slice(&self.id.to_le_bytes());
    }

    pub fn decode_header(bytes: &[u8]) -> Result<(u64, PageKind), String> {
        if bytes.len() < PAGE_SIZE {
            return Err("Page too small".into());
        }
        let magic = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        if magic != PAGE_MAGIC {
            return Err("Invalid page magic".into());
        }
        let kind = match bytes[4] {
            0 => PageKind::Free,
            1 => PageKind::Heap,
            2 => PageKind::Index,
            3 => PageKind::Meta,
            _ => return Err("Unknown page kind".into()),
        };
        let id = u64::from_le_bytes(bytes[5..13].try_into().unwrap());
        Ok((id, kind))
    }

    pub fn payload(&self) -> &[u8] {
        &self.data[16..]
    }

    pub fn payload_mut(&mut self) -> &mut [u8] {
        &mut self.data[16..]
    }
}

pub fn page_checksum(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc ^= b as u32;
        crc = crc.rotate_left(5);
    }
    crc
}
