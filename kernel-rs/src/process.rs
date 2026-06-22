//! Extended process model — Rust rewrite of `include/urte/process.h`.

use crate::error::{KernelError, Result};
use crate::types::{ScaleLevel, Trl};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcRole {
    Posix,
    RegenAgent,
    DigitalTwin,
    GuardrailMonitor,
}

#[derive(Debug, Clone)]
pub struct ProcInfo {
    pub pid: i32,
    pub ppid: i32,
    pub role: ProcRole,
    pub scale_level: ScaleLevel,
    pub trl_level: Trl,
    pub is_trl_pull_source: bool,
    pub state_vector: Option<i32>,
}

impl ProcInfo {
    pub fn new(pid: i32) -> Result<ProcInfo> {
        if pid <= 0 {
            return Err(KernelError::NoProc);
        }
        Ok(ProcInfo {
            pid,
            ppid: 0,
            role: ProcRole::Posix,
            scale_level: ScaleLevel::Molecular,
            trl_level: 0,
            is_trl_pull_source: false,
            state_vector: None,
        })
    }

    pub fn set_role(&mut self, role: ProcRole, scale: ScaleLevel, trl: Trl) {
        self.role = role;
        self.scale_level = scale;
        self.trl_level = trl;
    }
}
