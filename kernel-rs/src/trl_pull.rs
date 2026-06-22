//! TRL-Pull scheduling — Rust rewrite of `include/urte/trl_pull.h`.
//!
//! A high-maturity process notifies the scheduler of generated revenue/data;
//! dependent lower-TRL processes get a priority boost. The notifier's own
//! scheduling is never altered.

use std::collections::HashMap;

use crate::error::{KernelError, Result};

pub const TRL_REVENUE: u32 = 0x01;
pub const TRL_DATA: u32 = 0x02;

#[derive(Debug, Default)]
pub struct TrlPullScheduler {
    /// source pid -> dependent pids
    links: HashMap<i32, Vec<i32>>,
    /// pid -> accumulated priority boost
    boost: HashMap<i32, i64>,
}

impl TrlPullScheduler {
    pub fn new() -> Self {
        TrlPullScheduler::default()
    }

    pub fn link(&mut self, source_pid: i32, dependent_pid: i32) -> Result<()> {
        if source_pid <= 0 || dependent_pid <= 0 {
            return Err(KernelError::NoProc);
        }
        self.links.entry(source_pid).or_default().push(dependent_pid);
        Ok(())
    }

    /// Notify that `source_pid` produced `units` of revenue/data, boosting
    /// dependents. Returns the number of dependents boosted.
    pub fn notify(&mut self, source_pid: i32, units: u64, _flags: u32) -> Result<usize> {
        if source_pid <= 0 {
            return Err(KernelError::NoProc);
        }
        let boost = Self::calculate_boost(units);
        let deps = self.links.get(&source_pid).cloned().unwrap_or_default();
        for dep in &deps {
            *self.boost.entry(*dep).or_insert(0) += boost;
        }
        Ok(deps.len())
    }

    pub fn boost_of(&self, pid: i32) -> i64 {
        *self.boost.get(&pid).unwrap_or(&0)
    }

    fn calculate_boost(units: u64) -> i64 {
        // Logarithmic, saturating boost so large contributions don't starve
        // best-effort POSIX work.
        let b = (units as f64 + 1.0).ln().round() as i64;
        b.clamp(0, 20)
    }
}
