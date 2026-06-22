//! Flow-based stream processing — an Apache NiFi-style agent architecture.
//!
//! A [`FlowGraph`] is a chain of [`Processor`]s linked by bounded, back-pressured
//! connections (crossbeam channels). Each processor consumes a [`FlowFile`] and
//! emits zero or more FlowFiles — so a stage can drop, pass, or split records.
//! That fan-out is the "stream variability" captured by the [`ProvenanceRepo`]:
//! every processing event records how many outputs a record produced, giving a
//! NiFi-like data-provenance / lineage view of the stream.
//!
//! Stages run concurrently (one scoped thread each); bounded connections apply
//! back-pressure when a downstream stage falls behind.

use crossbeam_channel::bounded;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

static NEXT_FF_ID: AtomicU64 = AtomicU64::new(1);

/// A unit of stream data: attributes (metadata) + content + identity.
#[derive(Clone, Debug)]
pub struct FlowFile {
    pub id: u64,
    pub attributes: HashMap<String, String>,
    pub content: Vec<u8>,
}

impl FlowFile {
    pub fn new() -> Self {
        FlowFile {
            id: NEXT_FF_ID.fetch_add(1, Ordering::Relaxed),
            attributes: HashMap::new(),
            content: Vec::new(),
        }
    }
    pub fn attr(mut self, k: &str, v: &str) -> Self {
        self.attributes.insert(k.to_string(), v.to_string());
        self
    }
    pub fn get(&self, k: &str) -> Option<&String> {
        self.attributes.get(k)
    }
    pub fn with_content(mut self, c: Vec<u8>) -> Self {
        self.content = c;
        self
    }
}

impl Default for FlowFile {
    fn default() -> Self {
        FlowFile::new()
    }
}

/// A provenance/lineage event capturing what a processor did to one FlowFile.
#[derive(Clone, Debug)]
pub struct ProvenanceEvent {
    pub processor: String,
    pub flowfile_id: u64,
    /// Number of FlowFiles emitted: 0 = DROP, 1 = PASS, >1 = SPLIT. This is the
    /// captured stream variability.
    pub outputs: usize,
    pub note: String,
}

/// Shared, thread-safe provenance repository (the "data provenance" of NiFi).
#[derive(Default)]
pub struct ProvenanceRepo {
    events: Mutex<Vec<ProvenanceEvent>>,
}

impl ProvenanceRepo {
    pub fn record(&self, e: ProvenanceEvent) {
        self.events.lock().unwrap().push(e);
    }
    pub fn len(&self) -> usize {
        self.events.lock().unwrap().len()
    }
    pub fn is_empty(&self) -> bool {
        self.events.lock().unwrap().is_empty()
    }
    pub fn snapshot(&self) -> Vec<ProvenanceEvent> {
        self.events.lock().unwrap().clone()
    }
    /// (dropped, passed, split) counts — the stream-variability summary.
    pub fn variability(&self) -> (usize, usize, usize) {
        let g = self.events.lock().unwrap();
        let (mut dropped, mut passed, mut split) = (0, 0, 0);
        for e in g.iter() {
            match e.outputs {
                0 => dropped += 1,
                1 => passed += 1,
                _ => split += 1,
            }
        }
        (dropped, passed, split)
    }
}

/// A NiFi-style processor: consumes one FlowFile, emits zero or more.
pub trait Processor: Send + Sync {
    fn name(&self) -> &str;
    fn process(&self, ff: FlowFile) -> Vec<FlowFile>;
}

/// Build a processor from a closure (transform / filter / split).
pub struct FnProcessor {
    name: String,
    f: Box<dyn Fn(FlowFile) -> Vec<FlowFile> + Send + Sync>,
}

impl FnProcessor {
    pub fn new<F>(name: impl Into<String>, f: F) -> Arc<dyn Processor>
    where
        F: Fn(FlowFile) -> Vec<FlowFile> + Send + Sync + 'static,
    {
        Arc::new(FnProcessor { name: name.into(), f: Box::new(f) })
    }
}

impl Processor for FnProcessor {
    fn name(&self) -> &str {
        &self.name
    }
    fn process(&self, ff: FlowFile) -> Vec<FlowFile> {
        (self.f)(ff)
    }
}

/// A NiFi-style "PutSQL" sink: writes each FlowFile into a table of the SQL
/// schema, taking column values from the FlowFile's attributes. Passes the
/// FlowFile through on success, drops it on failure (captured as variability).
pub struct SqlSinkProcessor {
    name: String,
    mesh: crate::mesh::SqlMesh,
    table: String,
    columns: Vec<String>,
}

impl SqlSinkProcessor {
    pub fn new(
        name: impl Into<String>,
        mesh: crate::mesh::SqlMesh,
        table: impl Into<String>,
        columns: &[&str],
    ) -> Arc<dyn Processor> {
        Arc::new(SqlSinkProcessor {
            name: name.into(),
            mesh,
            table: table.into(),
            columns: columns.iter().map(|s| s.to_string()).collect(),
        })
    }
}

impl Processor for SqlSinkProcessor {
    fn name(&self) -> &str {
        &self.name
    }
    fn process(&self, ff: FlowFile) -> Vec<FlowFile> {
        let cols = self.columns.join(", ");
        let vals: Vec<String> = self
            .columns
            .iter()
            .map(|c| {
                let raw = ff.attributes.get(c).map(String::as_str).unwrap_or("");
                format!("'{}'", raw.replace('\'', "''")) // escape single quotes
            })
            .collect();
        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            self.table,
            cols,
            vals.join(", ")
        );
        match self.mesh.execute(&sql) {
            Ok(_) => vec![ff],  // pass through
            Err(_) => Vec::new(), // drop on failure
        }
    }
}

/// One stage in the flow: a processor plus its intra-stage concurrency (the
/// number of FlowFiles it may process in parallel, NiFi's "Concurrent Tasks").
struct Stage {
    processor: Arc<dyn Processor>,
    concurrency: usize,
}

/// A back-pressured chain of processors with provenance capture and
/// rayon-powered intra-stage parallelism.
pub struct FlowGraph {
    stages: Vec<Stage>,
    capacity: usize,
    default_concurrency: usize,
    /// Optional private rayon pool. When `Some`, intra-stage batches run on it
    /// (isolated from the global pool); when `None`, the global pool is used.
    pool: Option<Arc<rayon::ThreadPool>>,
    prov: Arc<ProvenanceRepo>,
}

impl FlowGraph {
    pub fn new(capacity: usize) -> Self {
        FlowGraph {
            stages: Vec::new(),
            capacity: capacity.max(1),
            // Default each stage to the logical-CPU (hyperthread) count.
            default_concurrency: num_cpus::get().max(1),
            pool: None,
            prov: Arc::new(ProvenanceRepo::default()),
        }
    }

    /// Attach a private rayon thread pool with `threads` worker threads. All
    /// intra-stage parallel processing then runs on this pool instead of the
    /// global one, isolating the flow's CPU use from the rest of the kernel.
    pub fn with_thread_pool(
        &mut self,
        threads: usize,
    ) -> std::result::Result<&mut Self, rayon::ThreadPoolBuildError> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads.max(1))
            .thread_name(|i| format!("urte-flow-{i}"))
            .build()?;
        self.pool = Some(Arc::new(pool));
        Ok(self)
    }

    /// Worker-thread count of the private pool, if one is attached.
    pub fn pool_threads(&self) -> Option<usize> {
        self.pool.as_ref().map(|p| p.current_num_threads())
    }

    /// Set the default intra-stage concurrency applied to stages added after
    /// this call.
    pub fn with_default_concurrency(&mut self, n: usize) -> &mut Self {
        self.default_concurrency = n.max(1);
        self
    }

    /// Add a stage using the default intra-stage concurrency.
    pub fn add(&mut self, p: Arc<dyn Processor>) -> &mut Self {
        let concurrency = self.default_concurrency;
        self.stages.push(Stage { processor: p, concurrency });
        self
    }

    /// Add a stage with an explicit intra-stage concurrency.
    pub fn add_parallel(&mut self, p: Arc<dyn Processor>, concurrency: usize) -> &mut Self {
        self.stages.push(Stage { processor: p, concurrency: concurrency.max(1) });
        self
    }

    pub fn provenance(&self) -> Arc<ProvenanceRepo> {
        Arc::clone(&self.prov)
    }

    /// Push `inputs` through the chain and collect what falls out of the last
    /// stage. Each stage runs on its own driver thread and bounded connections
    /// give back-pressure between stages; within a stage, a batch of up to
    /// `concurrency` FlowFiles is processed in parallel on the rayon pool.
    pub fn run(&self, inputs: Vec<FlowFile>) -> Vec<FlowFile> {
        if self.stages.is_empty() {
            return inputs;
        }
        let cap = self.capacity;

        std::thread::scope(|scope| {
            // Connection 0: feed inputs, then close by dropping the sender.
            let (tx0, rx0) = bounded::<FlowFile>(cap);
            scope.spawn(move || {
                for ff in inputs {
                    if tx0.send(ff).is_err() {
                        break;
                    }
                }
                // tx0 dropped here -> closes connection 0
            });

            // Chain each stage to the next via a fresh bounded connection.
            let mut rx = rx0;
            for stage in &self.stages {
                let (out_tx, out_rx) = bounded::<FlowFile>(cap);
                let in_rx = rx;
                let prov = Arc::clone(&self.prov);
                let proc = Arc::clone(&stage.processor);
                let pool = self.pool.clone();
                let batch_cap = stage.concurrency;
                scope.spawn(move || {
                    // Block for the first item, then greedily fill a batch.
                    while let Ok(first) = in_rx.recv() {
                        let mut batch = Vec::with_capacity(batch_cap);
                        batch.push(first);
                        while batch.len() < batch_cap {
                            match in_rx.try_recv() {
                                Ok(ff) => batch.push(ff),
                                Err(_) => break,
                            }
                        }

                        // Intra-stage data parallelism: process the batch on the
                        // private pool if attached, else the global rayon pool.
                        // `proc` is Send + Sync.
                        let work = || {
                            batch
                                .into_par_iter()
                                .map(|ff| {
                                    let id = ff.id;
                                    (id, proc.process(ff))
                                })
                                .collect::<Vec<(u64, Vec<FlowFile>)>>()
                        };
                        let processed = match &pool {
                            Some(p) => p.install(work),
                            None => work(),
                        };

                        for (id, outs) in processed {
                            prov.record(ProvenanceEvent {
                                processor: proc.name().to_string(),
                                flowfile_id: id,
                                outputs: outs.len(),
                                note: String::new(),
                            });
                            for o in outs {
                                if out_tx.send(o).is_err() {
                                    return;
                                }
                            }
                        }
                    }
                    // out_tx dropped here -> closes the next connection
                });
                rx = out_rx;
            }

            // Collector: drain the final connection.
            rx.iter().collect()
        })
    }
}
