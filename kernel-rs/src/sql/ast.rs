//! SQL abstract syntax tree for the URTE channel mesh API.
//!
//! A deliberately small SQL subset — `CREATE TABLE`, `INSERT`, `SELECT` with a
//! conjunctive `WHERE` — sufficient to drive the entity schema generated from
//! `model/schema.sql`.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Num(f64),
    Str(String),
    Null,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Num(n) => write!(f, "{n}"),
            Value::Str(s) => write!(f, "{s}"),
            Value::Null => write!(f, "NULL"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Predicate {
    pub column: String,
    pub op: CmpOp,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Projection {
    All,
    Columns(Vec<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    CreateTable {
        table: String,
        columns: Vec<String>,
    },
    Insert {
        table: String,
        columns: Vec<String>,
        values: Vec<Value>,
    },
    Select {
        table: String,
        projection: Projection,
        filters: Vec<Predicate>,
    },
}
