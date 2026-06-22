# urte-os/lib — Library dependencies & subsystem map

The URTE kernel library is the Rust crate at [`../kernel-rs`](../kernel-rs)
(`urte-kernel`). This folder documents the dependency set that turns it into an
RTOS-like, multithreaded, Elixir-style, NiFi-style streaming kernel, and maps
each subsystem to its module.

## External dependencies (Cargo)

| Crate | Version | Role in URTE |
|-------|---------|--------------|
| `crossbeam-channel` | 0.5 | Lock-free MPMC channels: actor mailboxes + NiFi back-pressured connections |
| `rayon` | 1 | Data-parallel / parallel-programming work-stealing pool (`parallel.rs`) |
| `num_cpus` | 1 | Hyperthread / logical-CPU detection (default parallelism width) |

Transitive (resolved by Cargo): `crossbeam-deque`, `crossbeam-epoch`,
`crossbeam-utils`, `rayon-core`, `either`, `libc`, `hermit-abi`.

> **Locking:** shared state uses `std::sync` (`Mutex`/`RwLock`). `parking_lot`
> was evaluated but its `windows-gnu` build needs MinGW `dlltool`, which the
> rustup GNU toolchain does not bundle. std locks cover every use here.

## Subsystem → module map

| Capability (requested) | Module | Notes |
|------------------------|--------|-------|
| **RTOS** real-time scheduling | `rtos.rs` | Fixed-priority + EDF tie-break, WCET & relative deadlines, deadline-miss accounting, utilization bound |
| **Multithreading / parallel** | `scheduler.rs`, `parallel.rs`, `sync.rs` | `sched_parallel` syscall (semaphore-bounded), rayon `par_map`/`par_reduce`, counting `Semaphore` |
| **Elixir-like process model** | `actor.rs` | GenServer-style `Actor`, mailboxes, `Addr` (pid), OTP `Supervisor` (one-for-one, let-it-crash restart) |
| **NiFi-like stream capture** | `flow.rs` | `Processor`/`FlowFile`, back-pressured connections, `ProvenanceRepo` capturing stream variability (drop/pass/split); **intra-stage parallelism** via rayon (`add_parallel`, default = CPU count) |
| **SQL-schema driven** | `sql/`, `mesh.rs`, `flow::SqlSinkProcessor` | Grammar + engine over `model/schema.sql`; flow sink writes FlowFiles into schema tables |
| POSIX core | `types/process/state_vector/guardrail/channel/trl_pull` | Original kernel primitives |
| UPL compiler/interpreter | `compiler/`, `interpreter/` | Pipeline DSL → IR → VM |

## Build & test

```sh
cd ../kernel-rs
cargo test          # 19 tests
cargo run --example run_pipeline
cargo run --example sql_mesh
```

Toolchain: `stable-x86_64-pc-windows-gnu` (self-contained linker; no Visual
Studio required). On Linux/macOS the default toolchain works unchanged.
