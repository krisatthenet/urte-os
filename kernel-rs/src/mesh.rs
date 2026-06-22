//! Channel request-call SQL mesh.
//!
//! The channel interface carries SQL request strings; the mesh executes them on
//! the in-kernel [`Engine`] and returns result sets. Concurrency is the union of
//! the three principles requested:
//!
//! * **semaphore** — a counting [`Semaphore`] bounds in-flight worker threads;
//! * **parallel scheduler syscall** — [`sched_parallel`] fans a batch of
//!   requests across threads (Rust hyperthreading);
//! * **RwLock engine** — `SELECT`s take a shared read guard and run truly in
//!   parallel; writes (`INSERT`/`CREATE`) take an exclusive write guard.

use std::sync::mpsc;
use std::sync::{Arc, RwLock};
use std::thread;

use crate::scheduler::sched_parallel;
use crate::sql::{self, Catalog, Engine, ResultSet, SqlError, Stmt};
use crate::sync::{OwnedPermit, Semaphore};

/// Execute one SQL request string against the shared engine, choosing the
/// correct lock: read-only batches run under a read guard (parallel), anything
/// with a write runs under a write guard (serialized).
fn run_on(engine: &RwLock<Engine>, sql: &str) -> Result<ResultSet, SqlError> {
    let stmts = sql::parse(sql)?;
    let read_only = stmts.iter().all(|s| matches!(s, Stmt::Select { .. }));

    if read_only {
        let guard = engine.read().expect("engine rwlock poisoned");
        let mut last = ResultSet { columns: Vec::new(), rows: Vec::new() };
        for s in stmts {
            if let Stmt::Select { table, projection, filters } = s {
                last = guard.select(&table, &projection, &filters)?;
            }
        }
        Ok(last)
    } else {
        let mut guard = engine.write().expect("engine rwlock poisoned");
        let mut last = ResultSet::status("OK");
        for s in stmts {
            last = guard.execute(s)?;
        }
        Ok(last)
    }
}

/// A request flowing over the mesh channel: a SQL string plus a reply channel.
pub struct MeshRequest {
    pub sql: String,
    pub reply: mpsc::Sender<Result<ResultSet, SqlError>>,
}

/// The SQL mesh node. Cheap to clone-share via `Arc`.
#[derive(Clone)]
pub struct SqlMesh {
    engine: Arc<RwLock<Engine>>,
    max_parallel: usize,
}

impl SqlMesh {
    pub fn new(catalog: Catalog, max_parallel: usize) -> Self {
        SqlMesh {
            engine: Arc::new(RwLock::new(Engine::new(catalog))),
            max_parallel: max_parallel.max(1),
        }
    }

    /// Mesh seeded with the URTE schema from `model/schema.sql`.
    pub fn urte_default(max_parallel: usize) -> Self {
        SqlMesh::new(Catalog::urte_default(), max_parallel)
    }

    /// Execute a single request synchronously on the caller's thread.
    pub fn execute(&self, sql: &str) -> Result<ResultSet, SqlError> {
        run_on(&self.engine, sql)
    }

    /// Fan a batch of requests across worker threads via the parallel scheduler
    /// syscall (bounded by the semaphore inside `sched_parallel`). Results are
    /// returned in submission order.
    pub fn execute_mesh(&self, requests: Vec<String>) -> Vec<Result<ResultSet, SqlError>> {
        let closures: Vec<_> = requests
            .into_iter()
            .map(|sql| {
                let engine = Arc::clone(&self.engine);
                move || run_on(&engine, &sql)
            })
            .collect();
        sched_parallel(closures, self.max_parallel)
    }

    /// Serve requests arriving on a channel until all senders are dropped. Each
    /// request is dispatched on its own worker thread, throttled by the
    /// semaphore so at most `max_parallel` execute concurrently.
    pub fn serve(&self, rx: mpsc::Receiver<MeshRequest>) {
        let sem = Arc::new(Semaphore::new(self.max_parallel));
        let mut handles = Vec::new();
        for req in rx {
            let engine = Arc::clone(&self.engine);
            let sem = Arc::clone(&sem);
            handles.push(thread::spawn(move || {
                let _permit = OwnedPermit::new(&sem); // bounded hyperthreading
                let res = run_on(&engine, &req.sql);
                let _ = req.reply.send(res);
            }));
        }
        for h in handles {
            let _ = h.join();
        }
    }

    /// Spawn `serve` on a background thread and return a client sender plus the
    /// server's join handle. Drop the sender to shut the mesh down.
    pub fn spawn_server(&self) -> (mpsc::Sender<MeshRequest>, thread::JoinHandle<()>) {
        let (tx, rx) = mpsc::channel::<MeshRequest>();
        let node = self.clone();
        let handle = thread::spawn(move || node.serve(rx));
        (tx, handle)
    }
}

/// Helper for a client: send one SQL request over the mesh channel and block
/// for its reply.
pub fn request(tx: &mpsc::Sender<MeshRequest>, sql: &str) -> Result<ResultSet, SqlError> {
    let (rtx, rrx) = mpsc::channel();
    tx.send(MeshRequest { sql: sql.to_string(), reply: rtx })
        .map_err(|_| SqlError::Parse("mesh server unavailable".into()))?;
    rrx.recv().map_err(|_| SqlError::Parse("mesh reply dropped".into()))?
}
