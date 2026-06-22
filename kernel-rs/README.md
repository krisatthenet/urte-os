# urte-kernel (Rust)

Rust rewrite of the URTE OS kernel, with two new layers added to the kernel
library: a **compiler** and an **interpreter** for the URTE Pipeline Language
(UPL). Supersedes the C interface in `../include/urte` + `../kernel/urte_core.c`
(kept for the POSIX `mq`/FFI reference).

## Layout

```
kernel-rs/
├── Cargo.toml
├── src/
│   ├── lib.rs            Crate root, re-exports, integration tests
│   ├── types.rs          ScaleLevel, Stage, Decision, Trl
│   ├── error.rs          KernelError (each variant maps to a POSIX errno)
│   ├── process.rs        Extended PCB / process roles
│   ├── state_vector.rs   Multi-scale state vectors
│   ├── guardrail.rs      Guardrail policy + evaluation
│   ├── channel.rs        Typed scale-aware IPC + guardrail mask
│   ├── trl_pull.rs       TRL-Pull scheduler
│   ├── sync.rs           Semaphore (POSIX sem principle) + RAII permit
│   ├── scheduler.rs      sched_parallel syscall (semaphore-bounded threads)
│   ├── task.rs           Process/task manager
│   ├── compiler/         UPL front end
│   │   ├── lexer.rs      source -> tokens
│   │   ├── parser.rs     tokens -> AST
│   │   ├── ast.rs        AST types
│   │   ├── ir.rs         IR (Module / Chunk / Op) + disassembler
│   │   └── mod.rs        compile(): lex -> parse -> codegen
│   ├── interpreter/
│   │   └── mod.rs        VM: executes IR against kernel state
│   ├── sql/              SQL grammar + in-kernel engine
│   │   ├── lexer.rs      SQL tokens (case-insensitive keywords)
│   │   ├── parser.rs     tokens -> SQL AST (CREATE/INSERT/SELECT+WHERE)
│   │   ├── ast.rs        SQL AST + Value type
│   │   ├── catalog.rs    schema catalog seeded from model/schema.sql
│   │   └── engine.rs     row store + execution (select &self / insert &mut)
│   └── mesh.rs           channel request-call SQL mesh
└── examples/
    ├── run_pipeline.rs
    └── sql_mesh.rs
```

## SQL request mesh (channels + semaphore + scheduler)

The POSIX channel interface now carries **SQL request strings**, executed by an
in-kernel engine over the entity schema generated from `model/schema.sql`
(tables `filter`, `heap_1`, `heap_2`, `shuffler`, `stream_ops`, `heap_controler`,
`divinity_buffer`, `trinity_buffer`, `class_1_2`).

Three coordination principles combine:

1. **Semaphore** (`sync::Semaphore`) — counting semaphore (`acquire`/`release`/
   `try_acquire`), the `sem_wait`/`sem_post` principle, used everywhere to bound
   concurrency.
2. **Parallel scheduler syscall** (`scheduler::sched_parallel`) — fans a batch of
   tasks across OS threads (Rust hyperthreading); the semaphore caps how many
   run at once. `task::TaskManager` schedules process tasks through it.
3. **RwLock engine** — `SELECT` requests take a shared read guard and run truly
   in parallel; `INSERT`/`CREATE` take an exclusive write guard.

```rust
use urte_kernel::{mesh, SqlMesh};

let node = SqlMesh::urte_default(4);                 // 4-wide hyperthreading
node.execute("INSERT INTO divinity_buffer (id) VALUES ('dv-1')")?;

// parallel batch over worker threads
let rows = node.execute_mesh(vec![
    "SELECT * FROM divinity_buffer".into(),
    "SELECT id FROM divinity_buffer WHERE id = 'dv-1'".into(),
]);

// channel request-call server
let (tx, server) = node.spawn_server();
let rs = mesh::request(&tx, "SELECT * FROM divinity_buffer")?;
drop(tx); server.join().unwrap();
```

Run it: `cargo run --example sql_mesh`.

### SQL grammar (supported subset)

```sql
CREATE TABLE <t> (<col>, ...);
INSERT INTO <t> (<col>, ...) VALUES (<val>, ...);
SELECT * | <col>, ...  FROM <t>  [WHERE <col> <op> <val> [AND ...]];
-- <op> ::= = | != | <> | < | <= | > | >=
-- <val> ::= number | 'string' | NULL
```

## Compiler + interpreter layers

The kernel library can now load **URTE Pipeline Language** programs that drive
the operational stages, scale, and guardrails taken straight from the MBSE
model.

```text
pipeline "therapy" {
    scale tissue;
    stage sensing;
    stage data_gathering;
    stage therapy_delivery_mitigation;
    guard release if magnitude > 0.5;
    emit "pipeline complete";
}
```

Flow: `source → lexer → parser → AST → codegen → IR (Module) → Interpreter`.

- **Compiler** (`compiler::compile`) returns an `ir::Module`; `Module::disassemble()`
  prints the opcodes.
- **Interpreter** (`interpreter::Interpreter`) executes each chunk against an
  environment of runtime values (e.g. `magnitude`). A guardrail that escalates
  (Ethics veto) halts the pipeline; the `RunResult` records the stages run, the
  log, and every guardrail decision.

Stages, scales, and intervention kinds are validated at compile time against the
enums generated from the model, so an unknown `stage foo;` is a compile error.

## RTOS / concurrency / streaming layers

Backed by real dependencies (`crossbeam-channel`, `rayon`, `num_cpus`; locking via
`std::sync`). See [`../lib/README.md`](../lib/README.md) for the dependency map.

- **RTOS** (`rtos.rs`) — `RealtimeScheduler`: fixed-priority scheduling with an
  Earliest-Deadline-First tie-break, per-task WCET and relative deadlines,
  deadline-miss accounting, and a utilization bound `U = Σ wcet/period`.
- **Parallel / multithreading** (`scheduler.rs`, `parallel.rs`, `sync.rs`) — the
  semaphore-bounded `sched_parallel` syscall, plus rayon `par_map`/`par_reduce`
  and `cpu_count()` for the hyperthread width.
- **Elixir/OTP actors** (`actor.rs`) — GenServer-style `Actor` trait, mailboxes
  over crossbeam channels, cloneable `Addr` (pid), and a `Supervisor`
  (one-for-one, "let it crash": a panic in `handle` restarts the actor from its
  factory; the mailbox survives).
- **NiFi-style streaming** (`flow.rs`) — `Processor`/`FlowFile`, back-pressured
  bounded connections, concurrent stages, and a `ProvenanceRepo` that captures
  stream **variability** (each event records 0=drop / 1=pass / >1=split). The
  `SqlSinkProcessor` writes FlowFiles into the SQL schema (`PutSQL`).
  Each stage also has **intra-stage parallelism**: a batch of up to
  `concurrency` FlowFiles is processed in parallel on the rayon pool
  (`add_parallel(proc, n)`; default = logical-CPU count), the NiFi "Concurrent
  Tasks" model. An optional **private rayon pool**
  (`graph.with_thread_pool(n)?`) isolates the flow's CPU use from the global
  pool; without it, the global pool is used.

```rust
use urte_kernel::{flow::{FlowGraph, FnProcessor, FlowFile}};

let mut g = FlowGraph::new(16);
g.add(FnProcessor::new("drop_odd", |ff| if ff.id % 2 == 0 { vec![ff] } else { vec![] }));
let out = g.run((0..10).map(|_| FlowFile::new()).collect());
let (dropped, passed, split) = g.provenance().variability();
```

## Build & test

```sh
cargo build
cargo test                       # 19 tests
cargo run --example run_pipeline
cargo run --example sql_mesh
cargo run --example rtos_flow     # RTOS -> actor -> NiFi flow -> SQL
```

Pure-Rust dependency tree — no C/MSVC needed; builds on the GNU toolchain.

## POSIX fidelity

Every fallible call returns `KernelError`; `KernelError::errno()` yields the
classic value (`EINVAL=22`, `EPERM=1`, `EBADF=9`, …) so a thin FFI shim can keep
the `-1`/`errno` contract of the C headers.

## Status

Builds and passes **19/19 tests** on `stable-x86_64-pc-windows-gnu` (rustc 1.96).
All three examples run. The POSIX core primitives (state vectors, guardrails,
channels) keep `ENOSYS`/`TODO` kernel-side mechanics; the compiler, interpreter,
RTOS, actor, parallel, SQL, and flow layers are functional.
