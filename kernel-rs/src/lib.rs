//! # urte-kernel
//!
//! Rust rewrite of the URTE OS kernel (originally `urte-os/include/urte` +
//! `kernel/urte_core.c`). Provides the POSIX-style core primitives plus two new
//! layers requested for the kernel library:
//!
//! * [`compiler`] — the URTE Pipeline Language (UPL) front end:
//!   source → lexer → parser → IR.
//! * [`interpreter`] — a virtual machine that executes compiled IR against
//!   kernel state (scales, stages, guardrails).
//!
//! All primitives keep POSIX failure semantics: every fallible call returns a
//! [`error::KernelError`] whose [`error::KernelError::errno`] maps to a classic
//! `errno`, mirroring the C ABI in `include/urte/*.h`.
//!
//! Everything is `std`-only and builds offline (`cargo build`, no network).

pub mod actor;
pub mod channel;
pub mod compiler;
pub mod error;
pub mod flow;
pub mod guardrail;
pub mod interpreter;
pub mod mesh;
pub mod parallel;
pub mod process;
pub mod rtos;
pub mod scheduler;
pub mod sql;
pub mod state_vector;
pub mod sync;
pub mod task;
pub mod trl_pull;
pub mod types;

pub use actor::{Actor, ActorSystem, Addr, Supervisor};
pub use error::{KernelError, Result};
pub use flow::{FlowFile, FlowGraph, Processor, ProvenanceRepo};
pub use mesh::SqlMesh;
pub use rtos::{RealtimeScheduler, RtPriority};
pub use sql::{Catalog, Engine, ResultSet, SqlError};
pub use sync::Semaphore;
pub use types::{Decision, ScaleLevel, Stage, Trl};

/// Library version, mirroring `URTE_CORE_VERSION_*` in `include/urte/syscalls.h`.
pub const VERSION_MAJOR: u32 = 1;
pub const VERSION_MINOR: u32 = 0;

/// Default concurrency bound (semaphore permits) for the parallel scheduler and
/// the SQL request mesh — i.e. the maximum number of worker threads in flight.
pub const DEFAULT_MAX_PARALLEL: usize = 3;

/// Convenience: compile UPL source and run it in one call.
pub fn compile_and_run(
    src: &str,
    env: &std::collections::HashMap<String, f64>,
) -> std::result::Result<Vec<interpreter::RunResult>, compiler::CompileError> {
    let module = compiler::compile(src)?;
    let mut vm = interpreter::Interpreter::default();
    Ok(vm.run_module(&module, env))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const SRC: &str = r#"
        // URTE therapy pipeline
        pipeline "therapy" {
            scale tissue;
            stage sensing;
            stage data_gathering;
            stage therapy_delivery_mitigation;
            guard release if magnitude > 0.5;
            emit "pipeline complete";
        }
    "#;

    #[test]
    fn compiles_to_ir() {
        let module = compiler::compile(SRC).expect("compile");
        assert_eq!(module.chunks.len(), 1);
        assert_eq!(module.chunks[0].name, "therapy");
        // 1 scale + 3 stages + 1 guard + 1 emit + halt = 7 ops
        assert_eq!(module.chunks[0].ops.len(), 7);
    }

    #[test]
    fn upl_drives_fabrication_pipeline() {
        use std::collections::HashMap;
        let src = r#"
            pipeline "fabricate" {
                scale molecular;
                stage sensing;
                stage crawling;
                stage selection;
                stage fabrication;
                stage assembly;
                stage dissemination;
                emit "fabrication run complete";
            }
        "#;
        let results = compile_and_run(src, &HashMap::new()).expect("run");
        assert_eq!(results[0].stages.len(), 6);
        assert!(!results[0].blocked);
        // the model's fabrication flow matches Stage::FABRICATION
        assert_eq!(results[0].stages.as_slice(), &Stage::FABRICATION[..]);
    }

    #[test]
    fn guardrail_blocks_high_magnitude_release() {
        let mut env = HashMap::new();
        env.insert("magnitude".to_string(), 0.9);
        let results = compile_and_run(SRC, &env).expect("run");
        assert!(results[0].blocked, "high-magnitude release must be blocked");
        assert_eq!(results[0].decisions[0].decision, Decision::EthicsVeto);
    }

    #[test]
    fn guardrail_allows_low_magnitude() {
        let mut env = HashMap::new();
        env.insert("magnitude".to_string(), 0.2);
        let results = compile_and_run(SRC, &env).expect("run");
        assert!(!results[0].blocked);
        // all three stages executed
        assert_eq!(results[0].stages.len(), 3);
    }

    #[test]
    fn channel_guardrail_rejects_oversize() {
        use channel::{Channel, ChannelAttr, GuardMask};
        let attr = ChannelAttr {
            scale_level: ScaleLevel::Tissue,
            required_trl: 0,
            guardrail_mask: GuardMask::ALL,
            maxmsg: 4,
            msgsize: 8,
        };
        let mut ch = Channel::open("/t", attr).unwrap();
        assert!(ch.send(b"this-is-too-long", 1, 0).is_err());
        assert!(ch.send(b"ok", 1, 0).is_ok());
    }

    #[test]
    fn semaphore_bounds_permits() {
        let s = Semaphore::new(2);
        assert!(s.try_acquire());
        assert!(s.try_acquire());
        assert!(!s.try_acquire()); // exhausted
        s.release();
        assert!(s.try_acquire());
    }

    #[test]
    fn parallel_scheduler_runs_all_in_order() {
        let tasks: Vec<_> = (0..8)
            .map(|i| move || i * i)
            .collect();
        let out = scheduler::sched_parallel(tasks, 3); // at most 3 concurrent
        assert_eq!(out, vec![0, 1, 4, 9, 16, 25, 36, 49]);
    }

    #[test]
    fn task_manager_schedules_tasks() {
        use task::{Task, TaskManager};
        let tasks = vec![
            Task::new(1, "sense", || "ok:sense".to_string()),
            Task::new(2, "compose", || "ok:compose".to_string()),
        ];
        let results = TaskManager::new(2).sched_parallel(tasks);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, 1);
        assert_eq!(results[1].output, "ok:compose");
    }

    #[test]
    fn sql_mesh_insert_then_select() {
        let mesh = SqlMesh::urte_default(DEFAULT_MAX_PARALLEL);
        mesh.execute("INSERT INTO divinity_buffer (id) VALUES ('dv-1')").unwrap();
        mesh.execute("INSERT INTO divinity_buffer (id) VALUES ('dv-2')").unwrap();
        let rs = mesh.execute("SELECT id FROM divinity_buffer WHERE id = 'dv-2'").unwrap();
        assert_eq!(rs.columns, vec!["id".to_string()]);
        assert_eq!(rs.rows.len(), 1);
    }

    #[test]
    fn sql_mesh_parallel_batch() {
        let mesh = SqlMesh::urte_default(DEFAULT_MAX_PARALLEL);
        // seed
        for n in 0..10 {
            mesh.execute(&format!("INSERT INTO trinity_buffer (id) VALUES ('t-{n}')")).unwrap();
        }
        // fan out parallel reads across worker threads
        let reqs: Vec<String> = (0..10)
            .map(|n| format!("SELECT id FROM trinity_buffer WHERE id = 't-{n}'"))
            .collect();
        let results = mesh.execute_mesh(reqs);
        assert_eq!(results.len(), 10);
        assert!(results.iter().all(|r| r.as_ref().map(|rs| rs.rows.len() == 1).unwrap_or(false)));
    }

    #[test]
    fn sql_mesh_channel_server() {
        let mesh = SqlMesh::urte_default(DEFAULT_MAX_PARALLEL);
        let (tx, server) = mesh.spawn_server();

        mesh::request(&tx, "INSERT INTO class_1_2 (id) VALUES ('c-1')").unwrap();
        let rs = mesh::request(&tx, "SELECT * FROM class_1_2").unwrap();
        assert_eq!(rs.rows.len(), 1);

        drop(tx); // shut the mesh server down
        let _ = server.join();
    }

    #[test]
    fn rtos_meets_deadlines_for_feasible_set() {
        use rtos::{RealtimeScheduler, RtCounter, RtPriority};
        let mut sched = RealtimeScheduler::new();
        let a = RtCounter::new();
        let b = RtCounter::new();
        let (a2, b2) = (a.clone(), b.clone());
        // U = 1/2 + 1/4 = 0.75 <= 1.0 -> feasible
        sched.spawn_periodic("control_loop", RtPriority::Critical, 2, 1, 2, move |_| a2.incr());
        sched.spawn_periodic("telemetry", RtPriority::Normal, 4, 1, 4, move |_| b2.incr());
        let report = sched.run(8);
        assert_eq!(report.deadline_misses, 0);
        assert_eq!(a.get(), 4); // ticks 0,2,4,6
        assert_eq!(b.get(), 2); // ticks 0,4
        assert!((report.utilization - 0.75).abs() < 1e-9);
    }

    #[test]
    fn actor_handles_messages() {
        use actor::{Actor, ActorSystem, Pid};
        use std::sync::mpsc;

        struct Accumulator {
            total: i64,
            out: mpsc::Sender<i64>,
        }
        impl Actor for Accumulator {
            type Msg = i64;
            fn handle(&mut self, msg: i64) {
                self.total += msg;
                let _ = self.out.send(self.total);
            }
        }

        let (otx, orx) = mpsc::channel();
        let system = ActorSystem::new();
        let addr = system.spawn(Accumulator { total: 0, out: otx });
        addr.send(10);
        addr.send(5);
        assert_eq!(orx.recv().unwrap(), 10);
        assert_eq!(orx.recv().unwrap(), 15);
        let _ = Pid::from(0u64); // Pid is u64
        addr.stop();
        system.join_all();
    }

    #[test]
    fn supervisor_restarts_on_panic() {
        use actor::{Actor, ActorSystem, Restart, Supervisor};
        use std::sync::mpsc;

        struct Flaky {
            out: mpsc::Sender<String>,
        }
        impl Actor for Flaky {
            type Msg = bool; // true => panic
            fn started(&mut self, _pid: u64) {
                let _ = self.out.send("started".into());
            }
            fn handle(&mut self, boom: bool) {
                if boom {
                    panic!("induced crash");
                }
                let _ = self.out.send("ok".into());
            }
        }

        let (otx, orx) = mpsc::channel();
        let system = ActorSystem::new();
        let factory = move || Flaky { out: otx.clone() };
        let addr = Supervisor::one_for_one(&system, factory, Restart::Transient, 3);

        assert_eq!(orx.recv().unwrap(), "started");
        addr.send(true); // crash -> supervised restart
        assert_eq!(orx.recv().unwrap(), "started"); // restarted
        addr.send(false); // works after restart
        assert_eq!(orx.recv().unwrap(), "ok");
        addr.stop();
        system.join_all();
    }

    #[test]
    fn flow_captures_stream_variability() {
        use flow::{FlowFile, FlowGraph, FnProcessor};

        let mut g = FlowGraph::new(8);
        // pass-through tagger
        g.add(FnProcessor::new("tag", |ff| vec![ff.attr("seen", "1")]));
        // filter: drop odd ids
        g.add(FnProcessor::new("evens_only", |ff| {
            if ff.id % 2 == 0 { vec![ff] } else { Vec::new() }
        }));
        // split: duplicate each survivor
        g.add(FnProcessor::new("duplicate", |ff| vec![ff.clone(), ff]));

        let inputs: Vec<FlowFile> = (0..10).map(|_| FlowFile::new()).collect();
        let out = g.run(inputs);

        let prov = g.provenance();
        assert!(prov.len() >= 10); // at least one event per input at stage 1
        let (dropped, _passed, split) = prov.variability();
        assert!(dropped > 0, "filter stage should drop some records");
        assert!(split > 0, "duplicate stage should split some records");
        // every surviving record was duplicated
        assert_eq!(out.len() % 2, 0);
    }

    #[test]
    fn flow_intra_stage_parallelism() {
        use flow::{FlowFile, FlowGraph, FnProcessor};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use std::time::{Duration, Instant};

        // A processor that blocks briefly; with intra-stage parallelism the
        // total wall time must be far less than (N * per-item time).
        let in_flight = Arc::new(AtomicUsize::new(0));
        let max_seen = Arc::new(AtomicUsize::new(0));
        let (ic, mc) = (Arc::clone(&in_flight), Arc::clone(&max_seen));

        let mut g = FlowGraph::new(64);
        g.add_parallel(
            FnProcessor::new("slow_map", move |ff| {
                let now = ic.fetch_add(1, Ordering::SeqCst) + 1;
                mc.fetch_max(now, Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(20));
                ic.fetch_sub(1, Ordering::SeqCst);
                vec![ff]
            }),
            8, // 8-way concurrent
        );

        let inputs: Vec<FlowFile> = (0..16).map(|_| FlowFile::new()).collect();
        let start = Instant::now();
        let out = g.run(inputs);
        let elapsed = start.elapsed();

        assert_eq!(out.len(), 16);
        // observed concurrency exceeded 1 -> genuinely parallel within the stage
        assert!(max_seen.load(Ordering::SeqCst) > 1, "stage did not run in parallel");
        // 16 items * 20ms sequential = 320ms; parallel should be well under
        assert!(elapsed < Duration::from_millis(250), "too slow: {elapsed:?}");
    }

    #[test]
    fn flow_private_thread_pool_caps_parallelism() {
        use flow::{FlowFile, FlowGraph, FnProcessor};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use std::time::Duration;

        let in_flight = Arc::new(AtomicUsize::new(0));
        let max_seen = Arc::new(AtomicUsize::new(0));
        let (ic, mc) = (Arc::clone(&in_flight), Arc::clone(&max_seen));

        let mut g = FlowGraph::new(64);
        // Private pool of 2 threads -> at most 2 process() calls in flight,
        // even though the stage requests 8-way concurrency.
        g.with_thread_pool(2).unwrap();
        assert_eq!(g.pool_threads(), Some(2));
        g.add_parallel(
            FnProcessor::new("slow_map", move |ff| {
                let now = ic.fetch_add(1, Ordering::SeqCst) + 1;
                mc.fetch_max(now, Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(15));
                ic.fetch_sub(1, Ordering::SeqCst);
                vec![ff]
            }),
            8,
        );

        let inputs: Vec<FlowFile> = (0..16).map(|_| FlowFile::new()).collect();
        let out = g.run(inputs);
        assert_eq!(out.len(), 16);
        // Private 2-thread pool must bound observed concurrency to 2.
        assert!(max_seen.load(Ordering::SeqCst) <= 2, "private pool did not cap parallelism");
        assert!(max_seen.load(Ordering::SeqCst) >= 1);
    }

    #[test]
    fn flow_sink_into_sql_schema() {
        use flow::{FlowFile, FlowGraph, SqlSinkProcessor};

        let mesh = SqlMesh::urte_default(DEFAULT_MAX_PARALLEL);
        let mut g = FlowGraph::new(8);
        g.add(SqlSinkProcessor::new("put_sql", mesh.clone(), "divinity_buffer", &["id"]));

        let inputs: Vec<FlowFile> = (0..5)
            .map(|n| FlowFile::new().attr("id", &format!("dv-{n}")))
            .collect();
        let out = g.run(inputs);
        assert_eq!(out.len(), 5); // all passed through

        let rs = mesh.execute("SELECT * FROM divinity_buffer").unwrap();
        assert_eq!(rs.rows.len(), 5);
    }

    #[test]
    fn parallel_utilities() {
        let squares = parallel::par_map(vec![1, 2, 3, 4], |x| x * x);
        assert_eq!(squares, vec![1, 4, 9, 16]);
        let sum = parallel::par_reduce(vec![1, 2, 3, 4], 0, |x| x, |a, b| a + b);
        assert_eq!(sum, 10);
        assert!(parallel::cpu_count() >= 1);
    }
}
