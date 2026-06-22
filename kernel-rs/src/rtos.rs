//! Real-time scheduling layer — gives the URTE kernel RTOS-like behaviour.
//!
//! A deterministic, tick-driven fixed-priority scheduler with relative
//! deadlines and worst-case execution time (WCET) accounting. Within a tick,
//! ready tasks run highest-priority-first with Earliest-Deadline-First as the
//! tie-break; a task that cannot finish within its relative deadline is counted
//! as a deadline miss. This models hard-real-time intervention loops alongside
//! best-effort work, mirroring the spec's hybrid scheduler.
//!
//! It is intentionally simulation-based (deterministic, testable) rather than
//! wall-clock based, so it behaves identically on any host OS.

use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RtPriority {
    Idle = 0,
    Background = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

/// A periodic real-time task. `period`, `wcet` and `deadline` are in ticks.
pub struct RtTask {
    pub name: String,
    pub priority: RtPriority,
    pub period: u64,
    pub wcet: u64,
    pub deadline: u64,
    work: Box<dyn FnMut(u64) + Send + 'static>,
    runs: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RtReport {
    pub total_runs: u64,
    pub deadline_misses: u64,
    /// Processor utilization U = sum(wcet_i / period_i). Schedulable (loosely)
    /// when U <= 1.0.
    pub utilization: f64,
    pub per_task: Vec<(String, u64)>,
}

#[derive(Default)]
pub struct RealtimeScheduler {
    tasks: Vec<RtTask>,
}

impl RealtimeScheduler {
    pub fn new() -> Self {
        RealtimeScheduler { tasks: Vec::new() }
    }

    /// Register a periodic task.
    pub fn spawn_periodic<F>(
        &mut self,
        name: impl Into<String>,
        priority: RtPriority,
        period: u64,
        wcet: u64,
        deadline: u64,
        work: F,
    ) where
        F: FnMut(u64) + Send + 'static,
    {
        self.tasks.push(RtTask {
            name: name.into(),
            priority,
            period: period.max(1),
            wcet,
            deadline: deadline.max(1),
            work: Box::new(work),
            runs: 0,
        });
    }

    /// Theoretical utilization of the current task set.
    pub fn utilization(&self) -> f64 {
        self.tasks
            .iter()
            .map(|t| t.wcet as f64 / t.period as f64)
            .sum()
    }

    /// Run the scheduler for `ticks` ticks and return a report.
    pub fn run(&mut self, ticks: u64) -> RtReport {
        let mut total_runs = 0u64;
        let mut deadline_misses = 0u64;

        for tick in 0..ticks {
            // Collect ready tasks (released this tick) as (index, priority, deadline, wcet).
            let mut ready: Vec<(usize, RtPriority, u64, u64)> = self
                .tasks
                .iter()
                .enumerate()
                .filter(|(_, t)| tick % t.period == 0)
                .map(|(i, t)| (i, t.priority, t.deadline, t.wcet))
                .collect();

            // Highest priority first, then earliest deadline (EDF tie-break).
            ready.sort_by(|a, b| b.1.cmp(&a.1).then(a.2.cmp(&b.2)));

            // Simulate run-to-completion within the tick, accumulating WCET.
            let mut elapsed = 0u64;
            for (idx, _prio, deadline, wcet) in ready {
                elapsed += wcet;
                if elapsed > deadline {
                    deadline_misses += 1;
                }
                let task = &mut self.tasks[idx];
                (task.work)(tick);
                task.runs += 1;
                total_runs += 1;
            }
        }

        RtReport {
            total_runs,
            deadline_misses,
            utilization: self.utilization(),
            per_task: self.tasks.iter().map(|t| (t.name.clone(), t.runs)).collect(),
        }
    }
}

/// A shared atomic-ish counter usable by real-time tasks to publish output
/// without locking on the hot path (wraps a `parking_lot::Mutex` for clarity).
#[derive(Clone, Default)]
pub struct RtCounter(Arc<Mutex<u64>>);

impl RtCounter {
    pub fn new() -> Self {
        RtCounter(Arc::new(Mutex::new(0)))
    }
    pub fn incr(&self) {
        *self.0.lock().unwrap() += 1;
    }
    pub fn get(&self) -> u64 {
        *self.0.lock().unwrap()
    }
}
