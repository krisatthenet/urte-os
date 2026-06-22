//! End-to-end demo of the new kernel layers working together:
//!   RTOS scheduler  ->  Elixir-style actor  ->  NiFi flow  ->  SQL schema.
//!
//! Run with: `cargo run --example rtos_flow`

use std::sync::mpsc;
use urte_kernel::{
    actor::{Actor, ActorSystem},
    flow::{FlowFile, FlowGraph, FnProcessor, SqlSinkProcessor},
    parallel,
    rtos::{RealtimeScheduler, RtCounter, RtPriority},
    SqlMesh, DEFAULT_MAX_PARALLEL,
};

/// An Elixir-style actor that counts the records it is told about.
struct Collector {
    count: u64,
    out: mpsc::Sender<u64>,
}
impl Actor for Collector {
    type Msg = u64;
    fn handle(&mut self, n: u64) {
        self.count += n;
        let _ = self.out.send(self.count);
    }
}

fn main() {
    println!("urte-kernel RTOS/flow demo — {} logical CPUs\n", parallel::cpu_count());

    // ---- RTOS layer: two periodic real-time tasks ----
    let mut sched = RealtimeScheduler::new();
    let ticks = RtCounter::new();
    let t2 = ticks.clone();
    sched.spawn_periodic("intervention_loop", RtPriority::Critical, 2, 1, 2, move |_| t2.incr());
    sched.spawn_periodic("telemetry", RtPriority::Normal, 4, 1, 4, |_| {});
    let report = sched.run(8);
    println!(
        "RTOS: runs={} deadline_misses={} utilization={:.2} (control loop fired {}x)",
        report.total_runs, report.deadline_misses, report.utilization, ticks.get()
    );

    // ---- NiFi flow: generate -> filter -> SQL sink (capture variability) ----
    let mesh = SqlMesh::urte_default(DEFAULT_MAX_PARALLEL);
    let mut graph = FlowGraph::new(16);
    graph
        .add(FnProcessor::new("enrich", |ff| vec![ff.attr("source", "rtos")]))
        .add(FnProcessor::new("drop_odd", |ff| {
            if ff.id % 2 == 0 { vec![ff] } else { Vec::new() }
        }))
        .add(SqlSinkProcessor::new("put_sql", mesh.clone(), "trinity_buffer", &["id"]));

    let inputs: Vec<FlowFile> = (0..12)
        .map(|n| FlowFile::new().attr("id", &format!("ff-{n}")))
        .collect();
    let survived = graph.run(inputs);

    let (dropped, passed, split) = graph.provenance().variability();
    println!(
        "\nFlow: {} records survived to SQL; provenance variability dropped={} passed={} split={}",
        survived.len(), dropped, passed, split
    );

    let rows = mesh.execute("SELECT * FROM trinity_buffer").unwrap().rows.len();
    println!("SQL: trinity_buffer now holds {rows} rows");

    // ---- Actor layer: report the survivor count to a supervised process ----
    let (otx, orx) = mpsc::channel();
    let system = ActorSystem::new();
    let addr = system.spawn(Collector { count: 0, out: otx });
    addr.send(survived.len() as u64);
    println!("\nActor: collector total = {}", orx.recv().unwrap());
    addr.stop();
    system.join_all();
}
