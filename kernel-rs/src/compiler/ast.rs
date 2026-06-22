//! Abstract syntax tree for the URTE Pipeline Language (UPL).
//!
//! A `.upl` program declares one or more pipelines; each pipeline is an ordered
//! list of statements that drive the kernel's operational stages, scale, and
//! guardrails. Example:
//!
//! ```text
//! pipeline "therapy" {
//!     scale tissue;
//!     stage sensing;
//!     stage data_gathering;
//!     stage therapy_delivery_mitigation;
//!     guard release if magnitude > 0.5;
//!     emit "pipeline complete";
//! }
//! ```

use crate::guardrail::InterventionKind;
use crate::types::{ScaleLevel, Stage};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmp {
    Gt,
    Ge,
    Lt,
    Le,
    Eq,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub pipelines: Vec<Pipeline>,
}

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub name: String,
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Scale(ScaleLevel),
    Stage(Stage),
    Guard {
        kind: InterventionKind,
        field: String,
        cmp: Cmp,
        value: f64,
    },
    Emit(String),
}
