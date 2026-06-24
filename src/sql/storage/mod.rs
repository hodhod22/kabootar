//! High-performance storage engine for Kabootar SQL (Phases 1–3).

pub mod btree;
pub mod buffer;
pub mod mvcc;
pub mod pages;
pub mod parallel;
pub mod partition;
pub mod persist_v2;
pub mod planner;
pub mod prepare;
pub mod row_store;
pub mod stats;

pub use btree::BPlusTree;
pub use buffer::BufferPool;
pub use mvcc::MvccState;
pub use pages::{Page, PageKind, PAGE_SIZE};
pub use partition::{parse_partition_clause, PartitionSpec};
pub use persist_v2::{flush_dirty_pages, incremental_checkpoint, is_binary_kdb, load_engine_v2, save_engine_v2};
pub use planner::{AccessMethod, PlanNode, QueryPlanner};
pub use prepare::{PreparedCache, PreparedQuery};
pub use row_store::{RowSlot, RowStore};
pub use stats::{ColumnStats, TableStats};
