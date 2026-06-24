//! Kabootar OS permissions — capability-based access control per process.

use std::collections::HashMap;

/// Capability string, e.g. `device:gpu-0`, `vfs:read:/apps`, `net:connect`, `*`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capability(pub String);

impl Capability {
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if s.is_empty() {
            return Err("capability cannot be empty".into());
        }
        Ok(Self(s.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn matches(&self, required: &str) -> bool {
        capability_matches(&self.0, required)
    }
}

pub fn capability_matches(granted: &str, required: &str) -> bool {
    if granted == "*" {
        return true;
    }
    if granted.ends_with('*') {
        let prefix = &granted[..granted.len() - 1];
        return required.starts_with(prefix);
    }
    granted == required
}

pub fn device_cap(device_id: &str) -> String {
    format!("device:{device_id}")
}

pub fn device_ioctl_cap(device_id: &str, op: &str) -> String {
    format!("device-ioctl:{device_id}:{op}")
}

pub fn vfs_read_cap(path: &str) -> String {
    format!("vfs:read:{path}")
}

pub fn vfs_write_cap(path: &str) -> String {
    format!("vfs:write:{path}")
}

pub const NET_CONNECT: &str = "net:connect";
pub const HOTPLUG: &str = "hotplug:register";
pub const PERM_ADMIN: &str = "perm:admin";

#[derive(Debug, Clone)]
pub struct PermissionSet {
    grants: HashMap<u64, Vec<Capability>>,
}

impl Default for PermissionSet {
    fn default() -> Self {
        let mut grants = HashMap::new();
        grants.insert(1, vec![Capability("*".into())]);
        Self { grants }
    }
}

impl PermissionSet {
    pub fn grant(&mut self, pid: u64, cap: &str) -> Result<(), String> {
        let cap = Capability::parse(cap)?;
        let entry = self.grants.entry(pid).or_default();
        if !entry.iter().any(|c| c == &cap) {
            entry.push(cap);
        }
        Ok(())
    }

    pub fn revoke(&mut self, pid: u64, cap: &str) -> Result<bool, String> {
        let cap = Capability::parse(cap)?;
        let Some(entry) = self.grants.get_mut(&pid) else {
            return Ok(false);
        };
        let before = entry.len();
        entry.retain(|c| c != &cap);
        Ok(entry.len() < before)
    }

    pub fn list(&self, pid: u64) -> Vec<String> {
        self.grants
            .get(&pid)
            .map(|v| v.iter().map(|c| c.as_str().to_string()).collect())
            .unwrap_or_default()
    }

    pub fn is_allowed(&self, pid: u64, required: &str) -> bool {
        if pid == 1 {
            return true;
        }
        self.grants
            .get(&pid)
            .map(|caps| caps.iter().any(|c| c.matches(required)))
            .unwrap_or(false)
    }

    pub fn require(&self, pid: u64, required: &str) -> Result<(), String> {
        if self.is_allowed(pid, required) {
            Ok(())
        } else {
            Err(format!("permission denied: {required} (pid {pid})"))
        }
    }

    pub fn inherit_from(&mut self, parent: u64, child: u64) {
        if let Some(parent_caps) = self.grants.get(&parent).cloned() {
            self.grants.insert(child, parent_caps);
        }
    }

    pub fn clear(&mut self, pid: u64) {
        if pid != 1 {
            self.grants.remove(&pid);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard_and_prefix() {
        assert!(capability_matches("device:gpu-*", "device:gpu-0"));
        assert!(!capability_matches("device:gpu-*", "device:net-0"));
        assert!(capability_matches("*", "vfs:write:/secret"));
    }

    #[test]
    fn init_always_allowed() {
        let mut ps = PermissionSet::default();
        assert!(ps.is_allowed(1, "device:usb-hid-0"));
        assert!(!ps.is_allowed(99, "device:usb-hid-0"));
        ps.grant(99, "device:usb-*").unwrap();
        assert!(ps.is_allowed(99, "device:usb-hid-0"));
    }
}
