//! Kernel synchronization primitives.
//!
//! `Semaphore` implements the POSIX counting-semaphore principle (`sem_wait` /
//! `sem_post` / `sem_trywait`) on top of a `Mutex` + `Condvar`. It is the
//! coordination primitive used by the process/task manager and the parallel
//! scheduler syscall to bound concurrency, and by the SQL request mesh to cap
//! the number of in-flight worker threads (hyperthreading throttle).

use std::sync::{Arc, Condvar, Mutex};

#[derive(Debug)]
pub struct Semaphore {
    count: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    /// Create a semaphore with `permits` initial permits.
    pub fn new(permits: usize) -> Self {
        Semaphore { count: Mutex::new(permits), cv: Condvar::new() }
    }

    /// P / wait / `sem_wait`: block until a permit is available, then take one.
    pub fn acquire(&self) {
        let mut c = self.count.lock().unwrap();
        while *c == 0 {
            c = self.cv.wait(c).unwrap();
        }
        *c -= 1;
    }

    /// `sem_trywait`: take a permit without blocking; returns false if none.
    pub fn try_acquire(&self) -> bool {
        let mut c = self.count.lock().unwrap();
        if *c > 0 {
            *c -= 1;
            true
        } else {
            false
        }
    }

    /// V / post / `sem_post`: return a permit and wake one waiter.
    pub fn release(&self) {
        let mut c = self.count.lock().unwrap();
        *c += 1;
        self.cv.notify_one();
    }

    /// Current number of available permits (`sem_getvalue`).
    pub fn available(&self) -> usize {
        *self.count.lock().unwrap()
    }
}

/// RAII permit: acquires on construction, releases on drop. Holds an `Arc` so it
/// can travel into a worker thread.
pub struct OwnedPermit {
    sem: Arc<Semaphore>,
}

impl OwnedPermit {
    /// Acquire a permit from `sem`, blocking until one is free.
    pub fn new(sem: &Arc<Semaphore>) -> OwnedPermit {
        sem.acquire();
        OwnedPermit { sem: Arc::clone(sem) }
    }
}

impl Drop for OwnedPermit {
    fn drop(&mut self) {
        self.sem.release();
    }
}
