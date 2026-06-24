//! Strategy 9 — Community-driven updates: feature flags + instant channel swap.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateChannel {
    Beta,
    Stable,
    Classic,
}

impl UpdateChannel {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "beta" => Some(Self::Beta),
            "stable" => Some(Self::Stable),
            "classic" => Some(Self::Classic),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Beta => "beta",
            Self::Stable => "stable",
            Self::Classic => "classic",
        }
    }
}

pub struct CommunityUpdates {
    channel: UpdateChannel,
    versions: Vec<String>,
    pointer: usize,
    swap_ms: u64,
}

impl Default for CommunityUpdates {
    fn default() -> Self {
        Self {
            channel: UpdateChannel::Stable,
            versions: vec![
                "2.1.0-classic".into(),
                "2.1.0-stable".into(),
                "2.1.1-beta".into(),
            ],
            pointer: 1,
            swap_ms: 3000,
        }
    }
}

impl CommunityUpdates {
    pub fn switch_channel(&mut self, ch: UpdateChannel) -> String {
        let start = std::time::Instant::now();
        self.channel = ch;
        self.pointer = match ch {
            UpdateChannel::Classic => 0,
            UpdateChannel::Stable => 1,
            UpdateChannel::Beta => 2,
        };
        self.swap_ms = start.elapsed().as_millis().max(1) as u64;
        self.active_version().to_string()
    }

    pub fn rollback(&mut self, steps: usize) -> String {
        if steps > 0 && self.pointer > 0 {
            self.pointer -= 1;
        }
        self.active_version().to_string()
    }

    pub fn active_version(&self) -> &str {
        &self.versions[self.pointer]
    }

    pub fn partition_swap_ms(&self) -> u64 {
        self.swap_ms
    }

    pub fn channel(&self) -> UpdateChannel {
        self.channel
    }
}
