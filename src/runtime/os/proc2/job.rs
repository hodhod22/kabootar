//! Job objects — process groups with resource limits.

#[derive(Debug, Clone)]
pub struct JobObject {
    pub id: u64,
    pub name: String,
    pub members: Vec<u64>,
    pub cpu_quota: u32,
}
