use cargo_arc::{Cargo, run};
use clap::Parser;

fn main() {
    let Cargo::Arc(args) = Cargo::parse();
    if let Err(e) = run(args) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
