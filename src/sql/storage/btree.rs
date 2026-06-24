//! In-memory B+tree for range-capable indexes (Phase 2).

use std::cmp::Ordering;

const ORDER: usize = 32;

#[derive(Debug, Clone)]
pub struct BPlusTree {
    root: usize,
    nodes: Vec<Node>,
    pub unique: bool,
}

#[derive(Debug, Clone)]
enum Node {
    Internal {
        keys: Vec<String>,
        children: Vec<usize>,
    },
    Leaf {
        keys: Vec<String>,
        values: Vec<Vec<usize>>,
        next: Option<usize>,
    },
}

impl BPlusTree {
    pub fn new(unique: bool) -> Self {
        let leaf = Node::Leaf {
            keys: Vec::new(),
            values: Vec::new(),
            next: None,
        };
        Self {
            root: 0,
            nodes: vec![leaf],
            unique,
        }
    }

    pub fn insert(&mut self, key: String, row_slot: usize) {
        let root = self.root;
        if let Some((split_key, new_right)) = self.insert_node(root, key, row_slot) {
            let old_root = root;
            let new_root = self.nodes.len();
            self.nodes.push(Node::Internal {
                keys: vec![split_key],
                children: vec![old_root, new_right],
            });
            self.root = new_root;
        }
    }

    pub fn remove(&mut self, key: &str, row_slot: usize) {
        self.remove_node(self.root, key, row_slot);
    }

    pub fn lookup_eq(&self, key: &str) -> Option<Vec<usize>> {
        let mut node_id = self.root;
        loop {
            match &self.nodes[node_id] {
                Node::Internal { keys, children } => {
                    let pos = keys.partition_point(|k| k.as_str() < key);
                    node_id = children[pos.min(children.len() - 1)];
                }
                Node::Leaf { keys, values, .. } => {
                    return keys
                        .iter()
                        .position(|k| k == key)
                        .map(|i| values[i].clone());
                }
            }
        }
    }

    pub fn range_scan(&self, min_key: Option<&str>, max_key: Option<&str>) -> Vec<(String, Vec<usize>)> {
        let mut out = Vec::new();
        let mut leaf = self.find_leaf(min_key);
        while let Some(id) = leaf {
            if let Node::Leaf { keys, values, next } = &self.nodes[id] {
                for (k, v) in keys.iter().zip(values.iter()) {
                    if let Some(min) = min_key {
                        if k.as_str() < min {
                            continue;
                        }
                    }
                    if let Some(max) = max_key {
                        if k.as_str() > max {
                            return out;
                        }
                    }
                    out.push((k.clone(), v.clone()));
                }
                leaf = *next;
            } else {
                break;
            }
        }
        out
    }

    pub fn all_entries(&self) -> Vec<(String, Vec<usize>)> {
        self.range_scan(None, None)
    }

    fn find_leaf(&self, key: Option<&str>) -> Option<usize> {
        let mut node_id = self.root;
        loop {
            match &self.nodes[node_id] {
                Node::Internal { keys, children } => {
                    let pos = match key {
                        Some(k) => keys.partition_point(|x| x.as_str() < k),
                        None => 0,
                    };
                    node_id = children[pos.min(children.len() - 1)];
                }
                Node::Leaf { .. } => return Some(node_id),
            }
        }
    }

    fn insert_node(&mut self, node_id: usize, key: String, row_slot: usize) -> Option<(String, usize)> {
        if let Node::Internal { keys, children } = self.nodes[node_id].clone() {
            let key_ref = key.as_str();
            let pos = keys.partition_point(|k| k.as_str() < key_ref);
            let child = children[pos.min(children.len() - 1)];
            if let Some((split_key, new_child)) = self.insert_node(child, key, row_slot) {
                if let Node::Internal { keys, children } = &mut self.nodes[node_id] {
                    let ins = keys.partition_point(|k| k.as_str() < split_key.as_str());
                    keys.insert(ins, split_key);
                    children.insert(ins + 1, new_child);
                    if keys.len() >= ORDER {
                        return self.split_internal(node_id);
                    }
                }
            }
            return None;
        }
        match &mut self.nodes[node_id] {
            Node::Leaf { keys, values, .. } => {
                let pos = keys.partition_point(|k| k < &key);
                if pos < keys.len() && keys[pos] == key {
                    if self.unique {
                        values[pos] = vec![row_slot];
                    } else if !values[pos].contains(&row_slot) {
                        values[pos].push(row_slot);
                    }
                } else {
                    keys.insert(pos, key);
                    values.insert(pos, vec![row_slot]);
                }
                if keys.len() > ORDER {
                    return self.split_leaf(node_id);
                }
                None
            }
            Node::Internal { .. } => None,
        }
    }

    fn split_leaf(&mut self, node_id: usize) -> Option<(String, usize)> {
        let Node::Leaf { keys, values, next } = self.nodes[node_id].clone() else {
            return None;
        };
        let mid = keys.len() / 2;
        let split_key = keys[mid].clone();
        let right_id = self.nodes.len();
        self.nodes[node_id] = Node::Leaf {
            keys: keys[..mid].to_vec(),
            values: values[..mid].to_vec(),
            next: Some(right_id),
        };
        self.nodes.push(Node::Leaf {
            keys: keys[mid..].to_vec(),
            values: values[mid..].to_vec(),
            next,
        });
        Some((split_key, right_id))
    }

    fn split_internal(&mut self, node_id: usize) -> Option<(String, usize)> {
        let Node::Internal { keys, children } = self.nodes[node_id].clone() else {
            return None;
        };
        let mid = keys.len() / 2;
        let split_key = keys[mid].clone();
        let right_id = self.nodes.len();
        self.nodes[node_id] = Node::Internal {
            keys: keys[..mid].to_vec(),
            children: children[..=mid].to_vec(),
        };
        self.nodes.push(Node::Internal {
            keys: keys[mid + 1..].to_vec(),
            children: children[mid + 1..].to_vec(),
        });
        Some((split_key, right_id))
    }

    fn remove_node(&mut self, node_id: usize, key: &str, row_slot: usize) {
        match &mut self.nodes[node_id] {
            Node::Leaf { keys, values, .. } => {
                if let Some(pos) = keys.iter().position(|k| k == key) {
                    values[pos].retain(|&s| s != row_slot);
                    if values[pos].is_empty() {
                        keys.remove(pos);
                        values.remove(pos);
                    }
                }
            }
            Node::Internal { keys, children } => {
                let pos = keys.partition_point(|k| k.as_str() < key);
                let child = children[pos.min(children.len() - 1)];
                self.remove_node(child, key, row_slot);
            }
        }
    }
}

pub fn cmp_keys(a: &str, b: &str) -> Ordering {
    a.cmp(b)
}
