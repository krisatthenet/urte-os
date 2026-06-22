//! In-kernel SQL execution engine over the schema catalog.
//!
//! Holds row storage per table (rows aligned to the catalog column order).
//! `select` takes `&self` (shared) so the request mesh can run reads in
//! parallel under an `RwLock` read guard; `insert`/`create_table` take `&mut
//! self` (a write guard).

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;

use crate::sql::ast::{CmpOp, Predicate, Projection, Stmt, Value};
use crate::sql::catalog::Catalog;

#[derive(Debug, Clone, PartialEq)]
pub enum SqlError {
    UnknownTable(String),
    UnknownColumn { table: String, column: String },
    Arity { expected: usize, found: usize },
    Parse(String),
    Empty,
}

impl fmt::Display for SqlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SqlError::UnknownTable(t) => write!(f, "unknown table '{t}'"),
            SqlError::UnknownColumn { table, column } => {
                write!(f, "unknown column '{column}' in table '{table}'")
            }
            SqlError::Arity { expected, found } => {
                write!(f, "column/value count mismatch: expected {expected}, found {found}")
            }
            SqlError::Parse(m) => write!(f, "sql parse error: {m}"),
            SqlError::Empty => write!(f, "empty SQL request"),
        }
    }
}

impl std::error::Error for SqlError {}

#[derive(Debug, Clone, PartialEq)]
pub struct ResultSet {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}

impl ResultSet {
    pub fn status(msg: &str) -> ResultSet {
        ResultSet { columns: vec!["status".into()], rows: vec![vec![Value::Str(msg.into())]] }
    }

    /// Render as a simple text table (for the demo / logs).
    pub fn render(&self) -> String {
        let mut out = self.columns.join(" | ");
        out.push('\n');
        for row in &self.rows {
            let line: Vec<String> = row.iter().map(|v| v.to_string()).collect();
            out.push_str(&line.join(" | "));
            out.push('\n');
        }
        out
    }
}

#[derive(Debug, Clone)]
pub struct Engine {
    catalog: Catalog,
    data: HashMap<String, Vec<Vec<Value>>>,
}

impl Engine {
    pub fn new(catalog: Catalog) -> Self {
        let mut data = HashMap::new();
        for name in catalog.table_names() {
            data.insert(name, Vec::new());
        }
        Engine { catalog, data }
    }

    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    /// Convenience single-threaded dispatch (takes `&mut self`).
    pub fn execute(&mut self, stmt: Stmt) -> Result<ResultSet, SqlError> {
        match stmt {
            Stmt::Select { table, projection, filters } => self.select(&table, &projection, &filters),
            Stmt::Insert { table, columns, values } => self.insert(&table, &columns, values),
            Stmt::CreateTable { table, columns } => self.create_table(&table, &columns),
        }
    }

    pub fn create_table(&mut self, table: &str, columns: &[String]) -> Result<ResultSet, SqlError> {
        let cols: Vec<&str> = columns.iter().map(|s| s.as_str()).collect();
        self.catalog.define(table, &cols);
        self.data.insert(table.to_string(), Vec::new());
        Ok(ResultSet::status(&format!("CREATE TABLE {table}")))
    }

    pub fn insert(&mut self, table: &str, columns: &[String], values: Vec<Value>) -> Result<ResultSet, SqlError> {
        let def = self
            .catalog
            .get(table)
            .ok_or_else(|| SqlError::UnknownTable(table.to_string()))?
            .clone();

        if columns.len() != values.len() {
            return Err(SqlError::Arity { expected: columns.len(), found: values.len() });
        }

        // Build a full row aligned to the table's column order; missing -> Null.
        let mut row = vec![Value::Null; def.columns.len()];
        for (col, val) in columns.iter().zip(values.into_iter()) {
            let idx = def
                .col_index(col)
                .ok_or_else(|| SqlError::UnknownColumn { table: table.to_string(), column: col.clone() })?;
            row[idx] = val;
        }
        self.data.entry(table.to_string()).or_default().push(row);
        Ok(ResultSet { columns: vec!["rows_affected".into()], rows: vec![vec![Value::Num(1.0)]] })
    }

    pub fn select(&self, table: &str, projection: &Projection, filters: &[Predicate]) -> Result<ResultSet, SqlError> {
        let def = self
            .catalog
            .get(table)
            .ok_or_else(|| SqlError::UnknownTable(table.to_string()))?;

        // Resolve projected column indices.
        let proj_idx: Vec<usize> = match projection {
            Projection::All => (0..def.columns.len()).collect(),
            Projection::Columns(cols) => {
                let mut v = Vec::with_capacity(cols.len());
                for c in cols {
                    let idx = def
                        .col_index(c)
                        .ok_or_else(|| SqlError::UnknownColumn { table: table.to_string(), column: c.clone() })?;
                    v.push(idx);
                }
                v
            }
        };
        let out_columns: Vec<String> = proj_idx.iter().map(|&i| def.columns[i].clone()).collect();

        // Pre-resolve filter column indices.
        let mut fidx = Vec::with_capacity(filters.len());
        for p in filters {
            let idx = def
                .col_index(&p.column)
                .ok_or_else(|| SqlError::UnknownColumn { table: table.to_string(), column: p.column.clone() })?;
            fidx.push((idx, p.op, &p.value));
        }

        let empty = Vec::new();
        let rows_in = self.data.get(table).unwrap_or(&empty);
        let mut rows = Vec::new();
        for row in rows_in {
            if fidx.iter().all(|(idx, op, val)| eval(&row[*idx], *op, *val)) {
                rows.push(proj_idx.iter().map(|&i| row[i].clone()).collect());
            }
        }
        Ok(ResultSet { columns: out_columns, rows })
    }
}

fn cmp_values(a: &Value, b: &Value) -> Option<Ordering> {
    match (a, b) {
        (Value::Num(x), Value::Num(y)) => x.partial_cmp(y),
        (Value::Str(x), Value::Str(y)) => Some(x.cmp(y)),
        (Value::Null, Value::Null) => Some(Ordering::Equal),
        _ => None,
    }
}

fn eval(a: &Value, op: CmpOp, b: &Value) -> bool {
    match cmp_values(a, b) {
        Some(ord) => match op {
            CmpOp::Eq => ord == Ordering::Equal,
            CmpOp::Ne => ord != Ordering::Equal,
            CmpOp::Lt => ord == Ordering::Less,
            CmpOp::Le => ord != Ordering::Greater,
            CmpOp::Gt => ord == Ordering::Greater,
            CmpOp::Ge => ord != Ordering::Less,
        },
        // Incomparable types: only "not equal" holds.
        None => matches!(op, CmpOp::Ne),
    }
}
