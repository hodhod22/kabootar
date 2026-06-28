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

    pub fn sandbox_active(&self) -> bool {
        self.sandbox_enabled
    }
}
