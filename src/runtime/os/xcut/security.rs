//! Security Reference Monitor — ACL, capabilities, sandbox.


#[derive(Debug, Clone)]
pub struct AclEntry {
    pub subject: String,
    pub object: String,
    pub rights: Vec<String>,
}

pub struct SecurityMonitor {
    acls: Vec<AclEntry>,
    audits: u64,
    sandbox_enabled: bool,
}

impl Default for SecurityMonitor {
    fn default() -> Self {
        Self {
            acls: vec![AclEntry {
                subject: "uid:0".into(),
                object: "*".into(),
                rights: vec!["*".into()],
            }],
            audits: 0,
            sandbox_enabled: true,
        }
    }
}

impl SecurityMonitor {
    pub fn audit(&mut self, pid: u64, action: &str, allowed: bool) {
        self.audits += 1;
        let _ = (pid, action, allowed);
    }

    pub fn check_acl(&self, subject: &str, object: &str, right: &str) -> bool {
        self.acls.iter().any(|a| {
            (a.subject == subject || a.subject == "*")
                && (a.object == object || a.object == "*")
                && (a.rights.iter().any(|r| r == "*" || r == right))
        })
    }

    /// True when a path-specific ACL exists (not only `*` wildcards).
    pub fn has_path_acl(&self, path: &str) -> bool {
        self.acls.iter().any(|a| a.object == path)
    }

    pub fn grant_acl(&mut self, subject: &str, object: &str, right: &str) {
        if let Some(existing) = self
            .acls
            .iter_mut()
            .find(|a| a.subject == subject && a.object == object)
        {
            if !existing.rights.iter().any(|r| r == right || r == "*") {
                existing.rights.push(right.to_string());
            }
            return;
        }
        self.acls.push(AclEntry {
            subject: subject.to_string(),
            object: object.to_string(),
            rights: vec![right.to_string()],
        });
    }

    pub fn revoke_acl(&mut self, subject: &str, object: &str) -> bool {
        let before = self.acls.len();
        self.acls
            .retain(|a| !(a.subject == subject && a.object == object));
        before != self.acls.len()
    }

    pub fn sandbox_active(&self) -> bool {
        self.sandbox_enabled
    }

    pub fn audit_count(&self) -> u64 {
        self.audits
    }
}
