//! Demonstrates the compiler + interpreter layers end to end.
//! Run with: `cargo run --example run_pipeline`

use std::collections::HashMap;
use urte_kernel::{compiler, interpreter::Interpreter};

const SRC: &str = r#"
pipeline "therapy" {
    scale tissue;
    stage sensing;
    stage data_gathering;
    stage measure_compose;
    stage therapy_delivery_mitigation;
    guard release if magnitude > 0.5;
    emit "therapy pipeline complete";
}
"#;

fn main() {
    println!("urte-kernel v{}.{}\n",
        urte_kernel::VERSION_MAJOR, urte_kernel::VERSION_MINOR);

    // --- compiler layer ---
    let module = match compiler::compile(SRC) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("compile failed: {e}");
            std::process::exit(1);
        }
    };
    println!("== IR disassembly ==\n{}", module.disassemble());

    // --- interpreter layer: two scenarios ---
    for magnitude in [0.2_f64, 0.9_f64] {
        let mut env = HashMap::new();
        env.insert("magnitude".to_string(), magnitude);

        let mut vm = Interpreter::default();
        let results = vm.run_module(&module, &env);

        println!("== run with magnitude = {magnitude} ==");
        for r in &results {
            println!("pipeline \"{}\"  final_scale={}  blocked={}",
                r.pipeline, r.final_scale, r.blocked);
            for line in &r.log {
                println!("   | {line}");
            }
        }
        println!();
    }
}
