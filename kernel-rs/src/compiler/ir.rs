//! Intermediate representation emitted by the UPL compiler and consumed by the
//! interpreter. A `Module` holds one `Chunk` per pipeline; a `Chunk` is a flat
//! list of `Op`s executed in order by the VM.

use crate::compiler::ast::Cmp;
use crate::guardrail::InterventionKind;
use crate::types::{ScaleLevel, Stage};

#[derive(Debug, Clone)]
pub enum Op {
    /// Set the active scale level for subsequent operations.
    SetScale(ScaleLevel),
    /// Execute an operational pipeline stage.
    RunStage(Stage),
    /// Evaluate a guardrail; on escalation the VM records an Ethics veto.
    Guard {
        kind: InterventionKind,
        field: String,
        cmp: Cmp,
        threshold: f64,
    },
    /// Emit a message into the run log.
    Emit(String),
    /// End of chunk.
    Halt,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub name: String,
    pub ops: Vec<Op>,
}

#[derive(Debug, Clone, Default)]
pub struct Module {
    pub chunks: Vec<Chunk>,
}

impl Module {
    /// Human-readable disassembly, useful for debugging the compiler output.
    pub fn disassemble(&self) -> String {
        let mut out = String::new();
        for chunk in &self.chunks {
            out.push_str(&format!("chunk \"{}\":\n", chunk.name));
            for (i, op) in chunk.ops.iter().enumerate() {
                out.push_str(&format!("  {i:04}  {op:?}\n"));
            }
        }
        out
    }
}
