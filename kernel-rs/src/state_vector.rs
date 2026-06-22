//! Multi-scale state vectors — Rust rewrite of `include/urte/state_vector.h`.

use crate::error::{KernelError, Result};
use crate::types::ScaleLevel;

/// Resilience / tipping-point metadata (R = -lambda_dom).
#[derive(Debug, Clone, Copy, Default)]
pub struct Resilience {
    pub lambda_dom: f64,
    pub variance: f64,
    pub lag1_autocorr: f64,
}

#[derive(Debug, Clone)]
pub struct StateVector {
    pub level: ScaleLevel,
    pub dimension: usize,
    pub data: Vec<f64>,
    pub resilience: Resilience,
    pub guardrail_enforced: bool,
    pub name: Option<String>,
}

impl StateVector {
    pub fn create(level: ScaleLevel, dimension: usize, name: Option<String>) -> Result<StateVector> {
        if dimension == 0 {
            return Err(KernelError::Invalid("dimension must be > 0".into()));
        }
        Ok(StateVector {
            level,
            dimension,
            data: vec![0.0; dimension],
            resilience: Resilience::default(),
            guardrail_enforced: true,
            name,
        })
    }

    pub fn resilience(&self) -> Resilience {
        self.resilience
    }

    /// Controlled cross-scale transition; validates coupling before applying.
    pub fn transition(&mut self, new_level: ScaleLevel, coupling_strength: f64) -> Result<()> {
        if !(0.0..=1.0).contains(&coupling_strength) {
            return Err(KernelError::Invalid("coupling_strength out of range".into()));
        }
        self.level = new_level;
        Ok(())
    }
}
