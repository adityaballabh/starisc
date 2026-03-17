use prover::prover::{prove, verify};
use vm::{execute, parse_file};

mod common;
use common::get_op_path;

#[test]
fn prove_and_verify() {
    let prog = parse_file(&get_op_path("limited_ops")).unwrap();
    let (trace, _) = execute(&prog).unwrap();
    let proof = prove(&prog, &trace).unwrap();
    assert!(verify(&prog, proof).is_ok());
}

#[test]
fn tampered_trace_rejected() {
    let prog = parse_file(&get_op_path("limited_ops")).unwrap();
    let (mut trace, _) = execute(&prog).unwrap();
    // corrupt r3 at instr 2
    trace[2].registers[3] = 500;
    // debug: prove will panic on constraint check
    // release: prove will succeed but verify will reject the proof
    let result = std::panic::catch_unwind(|| prove(&prog, &trace));
    match result {
        Err(_) => {}
        Ok(Err(_)) => {}
        Ok(Ok(proof)) => assert!(verify(&prog, proof).is_err()),
    }
}
