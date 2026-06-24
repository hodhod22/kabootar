//! Multi-version concurrency control (Phase 3).

use std::collections::HashMap;

pub type TxnId = u64;

#[derive(Debug, Clone, Copy, Default)]
pub struct RowVersion {
    pub created_by: TxnId,
    pub deleted_by: Option<TxnId>,
}

#[derive(Debug, Clone, Default)]
pub struct MvccState {
    pub next_txn: TxnId,
    pub active_txn: Option<TxnId>,
    pub snapshot_txn: Option<TxnId>,
    pub row_versions: HashMap<(String, usize), RowVersion>,
}

impl MvccState {
    pub fn begin(&mut self) -> TxnId {
        self.next_txn += 1;
        let id = self.next_txn;
        self.active_txn = Some(id);
        self.snapshot_txn = Some(id.saturating_sub(1));
        id
    }

    pub fn commit(&mut self) {
        self.active_txn = None;
        self.snapshot_txn = None;
    }

    pub fn rollback(&mut self) {
        if let Some(txn) = self.active_txn {
            self.row_versions.retain(|_, v| v.created_by != txn);
            for v in self.row_versions.values_mut() {
                if v.deleted_by == Some(txn) {
                    v.deleted_by = None;
                }
            }
        }
        self.active_txn = None;
        self.snapshot_txn = None;
    }

    pub fn mark_insert(&mut self, table: &str, slot: usize) {
        if let Some(txn) = self.active_txn {
            self.row_versions.insert(
                (table.to_string(), slot),
                RowVersion {
                    created_by: txn,
                    deleted_by: None,
                },
            );
        }
    }

    pub fn mark_delete(&mut self, table: &str, slot: usize) {
        if let Some(txn) = self.active_txn {
            let key = (table.to_string(), slot);
            if let Some(v) = self.row_versions.get_mut(&key) {
                v.deleted_by = Some(txn);
            } else {
                self.row_versions.insert(
                    key,
                    RowVersion {
                        created_by: 0,
                        deleted_by: Some(txn),
                    },
                );
            }
        }
    }

    pub fn in_transaction(&self) -> bool {
        self.active_txn.is_some()
    }

    pub fn is_visible(&self, table: &str, slot: usize) -> bool {
        let snap = self.snapshot_txn.unwrap_or(u64::MAX);
        let active = self.active_txn;
        let key = (table.to_string(), slot);
        if let Some(v) = self.row_versions.get(&key) {
            if v.created_by > snap && active != Some(v.created_by) {
                return false;
            }
            if let Some(del) = v.deleted_by {
                if del <= snap {
                    return false;
                }
                if active == Some(del) {
                    return true;
                }
                return false;
            }
        }
        true
    }
}
