//! Parallel scheduler — the `sched_parallel` syscall.
//!
//! Runs a batch of tasks across OS threads (Rust hyperthreading) while a
//! counting [`Semaphore`] caps how many run at once. All tasks are spawned, but
//! each must take a permit before executing, so at most `max_parallel` run
//! concurrently; the rest block in `acquire` until a permit is posted. Results
//! are returned in submission order.

use std::sync::Arc;
use std::thread;

use crate::sync::{OwnedPermit, Semaphore};

/// `sched_parallel(2)` — execute `tasks` with bounded parallelism.
///
/// Returns each task's value in the original order. Panics in a task propagate
/// as a panic on join (mirrors a faulting kernel task being surfaced).
pub fn sched_parallel<T, F>(tasks: Vec<F>, max_parallel: usize) -> Vec<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let sem = Arc::new(Semaphore::new(max_parallel.max(1)));
    let handles: Vec<_> = tasks
        .into_iter()
        .map(|task| {
            let sem = Arc::clone(&sem);
            thread::spawn(move || {
                let _permit = OwnedPermit::new(&sem); // P; V on drop
                task()
            })
        })
        .collect();

    handles
        .into_iter()
        .map(|h| h.join().expect("scheduled task panicked"))
        .collect()
}

/// A reusable parallel scheduler that keeps a fixed concurrency bound.
#[derive(Debug)]
pub struct ParallelScheduler {
    sem: Arc<Semaphore>,
}

impl ParallelScheduler {
    pub fn new(max_parallel: usize) -> Self {
        ParallelScheduler { sem: Arc::new(Semaphore::new(max_parallel.max(1))) }
    }

    /// Spawn a single task under the scheduler's semaphore. The returned join
    /// handle yields the task's value.
    pub fn spawn<T, F>(&self, task: F) -> thread::JoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let sem = Arc::clone(&self.sem);
        thread::spawn(move || {
            let _permit = OwnedPermit::new(&sem);
            task()
        })
    }

    /// Available permits right now.
    pub fn free_permits(&self) -> usize {
        self.sem.available()
    }
}
