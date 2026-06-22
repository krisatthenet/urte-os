//! URTE interpreter layer: a small virtual machine that executes compiled UPL
//! IR (`compiler::ir::Module`) against kernel state. Each chunk runs to `Halt`
//! or until a guardrail blocks it.
//!
//! Runtime field values (e.g. `magnitude`) are supplied via an environment so
//! the same compiled pipeline can be replayed against different scenarios.

use std::collections::HashMap;

use crate::compiler::ast::Cmp;
use crate::compiler::ir::{Chunk, Module, Op};
use crate::guardrail::{GuardrailPolicy, InterventionKind, InterventionRequest};
use crate::types::{Decision, ScaleLevel, Stage};

#[derive(Debug, Clone)]
pub struct GuardDecision {
    pub kind: InterventionKind,
    pub field: String,
    pub decision: Decision,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct RunResult {
    pub pipeline: String,
    pub final_scale: ScaleLevel,
    pub stages: Vec<Stage>,
    pub log: Vec<String>,
    pub decisions: Vec<GuardDecision>,
    pub blocked: bool,
}

pub struct Interpreter {
    policy: GuardrailPolicy,
    scale: ScaleLevel,
}

impl Default for Interpreter {
    fn default() -> Self {
        Interpreter::new(GuardrailPolicy::default())
    }
}

impl Interpreter {
    pub fn new(policy: GuardrailPolicy) -> Self {
        Interpreter { policy, scale: ScaleLevel::Molecular }
    }

    /// Run every chunk in the module, returning one result per pipeline.
    pub fn run_module(&mut self, module: &Module, env: &HashMap<String, f64>) -> Vec<RunResult> {
        module.chunks.iter().map(|c| self.run_chunk(c, env)).collect()
    }

    /// Execute a single chunk.
    pub fn run_chunk(&mut self, chunk: &Chunk, env: &HashMap<String, f64>) -> RunResult {
        self.scale = ScaleLevel::Molecular;
        let mut res = RunResult {
            pipeline: chunk.name.clone(),
            final_scale: self.scale,
            stages: Vec::new(),
            log: Vec::new(),
            decisions: Vec::new(),
            blocked: false,
        };

        for op in &chunk.ops {
            match op {
                Op::SetScale(lvl) => {
                    self.scale = *lvl;
                    res.log.push(format!("scale := {lvl}"));
                }
                Op::RunStage(st) => {
                    res.stages.push(*st);
                    res.log.push(format!("run stage: {}", st.label()));
                }
                Op::Emit(s) => res.log.push(format!("emit: {s}")),
                Op::Guard { kind, field, cmp, threshold } => {
                    let value = *env.get(field).unwrap_or(&0.0);
                    if compare(value, *cmp, *threshold) {
                        let magnitude = *env.get("magnitude").unwrap_or(&value);
                        let req = InterventionRequest {
                            kind: *kind,
                            scale: self.scale,
                            magnitude: magnitude.clamp(0.0, 1.0),
                            location: format!("scale:{}", self.scale),
                        };
                        match self.policy.check(&req) {
                            Ok(gr) => {
                                res.decisions.push(GuardDecision {
                                    kind: *kind,
                                    field: field.clone(),
                                    decision: gr.decision,
                                    reason: gr.reason.clone(),
                                });
                                res.log.push(format!(
                                    "guard {:?} ({} {} {:.3}) -> {:?}",
                                    kind, field, cmp_str(*cmp), threshold, gr.decision
                                ));
                                if gr.decision != Decision::Allow {
                                    res.blocked = true;
                                    res.log.push("pipeline blocked by guardrail".into());
                                    break;
                                }
                            }
                            Err(e) => {
                                res.log.push(format!("guard error: {e}"));
                                res.blocked = true;
                                break;
                            }
                        }
                    } else {
                        res.log.push(format!(
                            "guard {:?} ({} {} {:.3}) -> condition false, skipped",
                            kind, field, cmp_str(*cmp), threshold
                        ));
                    }
                }
                Op::Halt => break,
            }
        }

        res.final_scale = self.scale;
        res
    }
}

fn compare(lhs: f64, cmp: Cmp, rhs: f64) -> bool {
    match cmp {
        Cmp::Gt => lhs > rhs,
        Cmp::Ge => lhs >= rhs,
        Cmp::Lt => lhs < rhs,
        Cmp::Le => lhs <= rhs,
        Cmp::Eq => (lhs - rhs).abs() < f64::EPSILON,
    }
}

fn cmp_str(cmp: Cmp) -> &'static str {
    match cmp {
        Cmp::Gt => ">",
        Cmp::Ge => ">=",
        Cmp::Lt => "<",
        Cmp::Le => "<=",
        Cmp::Eq => "==",
    }
}
