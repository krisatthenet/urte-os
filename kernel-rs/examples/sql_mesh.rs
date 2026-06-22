//! Demonstrates the channel SQL request mesh: a semaphore-bounded, multi-threaded
//! request-call API over the entity schema from `model/schema.sql`.
//!
//! Run with: `cargo run --example sql_mesh`

use urte_kernel::{mesh, SqlMesh, DEFAULT_MAX_PARALLEL};

fn main() {
    // Mesh seeded with the URTE schema (filter, heap_1/2, stream_ops, ...),
    // at most DEFAULT_MAX_PARALLEL (=3) worker threads in flight
    // (semaphore-bounded hyperthreading).
    let node = SqlMesh::urte_default(DEFAULT_MAX_PARALLEL);

    // --- writes (serialized under the engine write lock) ---
    for n in 0..6 {
        node.execute(&format!(
            "INSERT INTO stream_ops (id, heap_controler_id, divinity_buffer_id, trinity_buffer_id) \
             VALUES ('so-{n}', 'hc-{n}', 'dv-{n}', 'tr-{n}')"
        ))
        .unwrap();
    }

    // --- parallel reads fanned across worker threads via the scheduler syscall ---
    let requests: Vec<String> = (0..6)
        .map(|n| format!("SELECT id, heap_controler_id FROM stream_ops WHERE id = 'so-{n}'"))
        .collect();

    println!("== parallel mesh batch (6 reads, max 3 concurrent) ==");
    for (i, res) in node.execute_mesh(requests).into_iter().enumerate() {
        match res {
            Ok(rs) => print!("[{i}] {}", rs.render()),
            Err(e) => println!("[{i}] error: {e}"),
        }
    }

    // --- channel server: client sends SQL strings over an mpsc channel ---
    println!("\n== channel request-call server ==");
    let (tx, server) = node.spawn_server();
    let rs = mesh::request(&tx, "SELECT * FROM stream_ops WHERE id >= 'so-3'").unwrap();
    println!("rows with id >= 'so-3': {}", rs.rows.len());
    print!("{}", rs.render());

    drop(tx); // closing the channel shuts the mesh server down
    server.join().unwrap();
}
