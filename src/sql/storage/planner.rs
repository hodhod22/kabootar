//! Cost-based query planner (Phase 3).

use crate::sql::schema::TableDef;
use crate::sql::storage::stats::TableStats;

#[derive(Debug, Clone, PartialEq)]
pub enum AccessMethod {
    SeqScan,
    IndexScan { index: String },
    IndexOnlyScan { index: String },
}

#[derive(Debug, Clone)]
pub struct PlanNode {
    pub method: AccessMethod,
    pub table: String,
    pub estimated_rows: f64,
    pub estimated_cost: f64,
    pub index_name: Option<String>,
}

pub struct QueryPlanner;

impl QueryPlanner {
    pub fn plan_point_lookup(
        table: &TableDef,
        table_name: &str,
        column: &str,
        stats: &TableStats,
        select_columns: &[String],
    ) -> PlanNode {
        let rows = stats.row_count.max(table.live_row_count() as u64) as f64;
        let index_only = select_columns.len() == 1 && select_columns[0] == column;
        if table.primary_key.as_deref() == Some(column) {
            return PlanNode {
                method: if index_only {
                    AccessMethod::IndexOnlyScan {
                        index: "PRIMARY".into(),
                    }
                } else {
                    AccessMethod::IndexScan {
                        index: "PRIMARY".into(),
                    }
                },
                table: table_name.into(),
                estimated_rows: 1.0,
                estimated_cost: 1.0,
                index_name: Some("PRIMARY".into()),
            };
        }
        for idx in &table.indexes {
            if idx.columns == [column] || idx.columns.first().map(|s| s.as_str()) == Some(column) {
                return PlanNode {
                    method: if index_only {
                        AccessMethod::IndexOnlyScan {
                            index: idx.name.clone(),
                        }
                    } else {
                        AccessMethod::IndexScan {
                            index: idx.name.clone(),
                        }
                    },
                    table: table_name.into(),
                    estimated_rows: 1.0,
                    estimated_cost: 2.0,
                    index_name: Some(idx.name.clone()),
                };
            }
        }
        PlanNode {
            method: AccessMethod::SeqScan,
            table: table_name.into(),
            estimated_rows: rows,
            estimated_cost: rows * 0.01 + 10.0,
            index_name: None,
        }
    }

    pub fn plan_seq_scan(table_name: &str, stats: &TableStats, live_rows: usize) -> PlanNode {
        let rows = stats.row_count.max(live_rows as u64) as f64;
        PlanNode {
            method: AccessMethod::SeqScan,
            table: table_name.into(),
            estimated_rows: rows,
            estimated_cost: rows * 0.01 + 10.0,
            index_name: None,
        }
    }

    pub fn format_plan(node: &PlanNode) -> String {
        match &node.method {
            AccessMethod::SeqScan => format!("Seq Scan on {} (rows≈{:.0})", node.table, node.estimated_rows),
            AccessMethod::IndexScan { index } => {
                format!("Index Scan on {} using {} (rows≈{:.0})", node.table, index, node.estimated_rows)
            }
            AccessMethod::IndexOnlyScan { index } => format!(
                "Index Only Scan on {} using {} (rows≈{:.0})",
                node.table, index, node.estimated_rows
            ),
        }
    }
}
