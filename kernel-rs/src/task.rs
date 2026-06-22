//! Process/task manager.
//!
//! A `Task` is a named unit of work (a boxed closure). `TaskManager` dispatches
//! a set of tasks through the parallel scheduler syscall, so concurrency is
//! bounded by the same counting-semaphore principle used kernel-wide.

use crate::scheduler::sched_parallel;

/// A schedulable unit of work that produces a string result.
pub struct Task {
    pub id: u64,
    pub name: String,
    work: Box<dyn FnOnce() -> String + Send + 'static>,
}

impl Task {
    pub fn new<F>(id: u64, name: impl Into<String>, work: F) -> Task
    where
        F: FnOnce() -> String + Send + 'static,
    {
        Task { id, name: name.into(), work: Box::new(work) }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskResult {
    pub id: u64,
    pub name: String,
    pub output: String,
}

#[derive(Debug, Clone)]
pub struct TaskManager {
    max_parallel: usize,
}

impl TaskManager {
    pub fn new(max_parallel: usize) -> Self {
        TaskManager { max_parallel }
    }

    /// Run all tasks via the parallel scheduler syscall, bounded by the
    /// semaphore. Results come back in submission order.
    pub fn sched_parallel(&self, tasks: Vec<Task>) -> Vec<TaskResult> {
        let closures: Vec<_> = tasks
            .into_iter()
            .map(|t| {
                move || {
                    let Task { id, name, work } = t;
                    let output = work();
                    TaskResult { id, name, output }
                }
            })
            .collect();
        sched_parallel(closures, self.max_parallel)
    }
}
