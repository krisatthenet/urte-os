//! Kernel guardrail enforcement — Rust rewrite of `include/urte/guardrail.h`.
//!
//! Guardrail checks are performed IN ADDITION to standard permission checks and
//! cannot be bypassed. The default posture is conservative: irreversible
//! release or high-magnitude actuation escalates to an Ethics Board veto.

use crate::error::{KernelError, Result};
use crate::types::{Decision, ScaleLevel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterventionKind {
    Observe,
    Simulate,
    Actuate,
    Release,
}

#[derive(Debug, Clone)]
pub struct InterventionRequest {
    pub kind: InterventionKind,
    pub scale: ScaleLevel,
    pub magnitude: f64, // normalized 0..1
    pub location: String,
}

#[derive(Debug, Clone)]
pub struct GuardrailResult {
    pub decision: Decision,
    pub reason_code: i32,
    pub reason: String,
    pub ethics_board_required: bool,
}

/// Policy thresholds. A real deployment would load these as immutable kernel
/// policy; here they are explicit and auditable.
#[derive(Debug, Clone)]
pub struct GuardrailPolicy {
    pub magnitude_escalation: f64,
}

impl Default for GuardrailPolicy {
    fn default() -> Self {
        GuardrailPolicy { magnitude_escalation: 0.5 }
    }
}

impl GuardrailPolicy {
    /// Synchronously evaluate a proposed intervention.
    pub fn check(&self, req: &InterventionRequest) -> Result<GuardrailResult> {
        if !(0.0..=1.0).contains(&req.magnitude) {
            return Err(KernelError::Invalid("magnitude out of range".into()));
        }

        if req.kind == InterventionKind::Release || req.magnitude > self.magnitude_escalation {
            Ok(GuardrailResult {
                decision: Decision::EthicsVeto,
                reason_code: 1,
                reason: format!(
                    "escalation: kind={:?} magnitude={:.3} at scale {}",
                    req.kind, req.magnitude, req.scale
                ),
                ethics_board_required: true,
            })
        } else {
            Ok(GuardrailResult {
                decision: Decision::Allow,
                reason_code: 0,
                reason: "within policy".into(),
                ethics_board_required: false,
            })
        }
    }
}
