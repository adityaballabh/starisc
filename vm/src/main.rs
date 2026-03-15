use std::fs;
use vm::{dump_trace, execute, parse_file};

fn main() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../examples/sample.op");

    let prog = parse_file(path).unwrap_or_else(|e| {
        eprintln!("parse error: {}", e);
        std::process::exit(1);
    });

    let (trace, regs) = execute(&prog).unwrap_or_else(|e| {
        eprintln!("{}", e);
        std::process::exit(1);
    });

    fs::create_dir_all("logs").unwrap();
    dump_trace(&prog, &trace, &regs, "logs/trace.log").unwrap();
    println!("wrote to logs/trace.log");
}
