//! Schema catalog. `Catalog::urte_default()` is seeded directly from
//! `model/schema.sql` (the entity schema extracted from the Capella MBSE model),
//! so the channel SQL mesh exposes exactly those tables and columns.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct TableDef {
    pub name: String,
    pub columns: Vec<String>,
}

impl TableDef {
    pub fn col_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c == name)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Catalog {
    tables: HashMap<String, TableDef>,
}

impl Catalog {
    pub fn new() -> Self {
        Catalog::default()
    }

    pub fn define(&mut self, name: &str, columns: &[&str]) {
        let def = TableDef {
            name: name.to_string(),
            columns: columns.iter().map(|s| s.to_string()).collect(),
        };
        self.tables.insert(name.to_string(), def);
    }

    pub fn get(&self, name: &str) -> Option<&TableDef> {
        self.tables.get(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.tables.contains_key(name)
    }

    pub fn table_names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.tables.keys().cloned().collect();
        v.sort();
        v
    }

    /// Catalog matching `model/schema.sql` (URTE DATA schema package + Data Vault).
    pub fn urte_default() -> Self {
        let mut c = Catalog::new();
        // Data Vault
        c.define("divinity_buffer", &["id"]);
        c.define("trinity_buffer", &["id"]);
        c.define("class_1_2", &["id"]);
        c.define("heap_controler", &["id", "divinity_buffer_id", "trinity_buffer_id"]);
        // DATA schema package
        c.define("stream_ops", &["id", "heap_controler_id", "divinity_buffer_id", "trinity_buffer_id"]);
        c.define("shuffler", &["id", "stream_ops_id"]);
        c.define("heap_2", &["id", "shuffler_id", "stream_ops_id"]);
        c.define("heap_1", &["id", "heap2_id", "stream_ops_id"]);
        c.define("filter", &["id", "heap1_id", "stream_ops_id"]);
        c
    }
}
