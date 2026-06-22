//! SQL layer for the URTE kernel: grammar (lexer + parser), schema catalog, and
//! an in-kernel execution engine. The catalog is seeded from `model/schema.sql`.
//!
//! Consumed by [`crate::mesh`], which exposes it as a channel request-call API.

pub mod ast;
pub mod catalog;
pub mod engine;
pub mod lexer;
pub mod parser;

pub use ast::{CmpOp, Predicate, Projection, Stmt, Value};
pub use catalog::{Catalog, TableDef};
pub use engine::{Engine, ResultSet, SqlError};

/// Parse SQL source into statements (grammar entry point).
pub fn parse(src: &str) -> Result<Vec<Stmt>, SqlError> {
    parser::parse(src).map_err(|e| SqlError::Parse(e.msg))
}
