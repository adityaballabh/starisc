use prover::prover::{prove, verify};
use vm::{execute, parse_file};

use test_utils::{assert_proof_rejected, get_op_path};

const ADD_INSTR: usize = 2;
const LT_INSTR: usize = 7;
const ADD_RES_REG: usize = 7;
const LT_RES_REG: usize = 8;

#[test]
fn prove_and_verify() {
    let prog = parse_file(&get_op_path("limited_ops")).unwrap();
    let (trace, _) = execute(&prog).unwrap();
    assert!(verify(&prog, prove(&prog, &trace).unwrap()).is_ok());
}

fn assert_tamper_rejection(prog: &[vm::Instruction], trace: &vm::Trace) {
    let (trace_clone, prog_vec) = (trace.clone(), prog.to_vec());
    assert_proof_rejected(
        move || prove(&prog_vec, &trace_clone),
        |proof| verify(prog, proof),
    );
}

#[test]
fn rejects_tampered_add() {
    let prog = parse_file(&get_op_path("limited_ops")).unwrap();
    let (mut trace, _) = execute(&prog).unwrap();
    trace[ADD_INSTR].registers[ADD_RES_REG] += 10;
    assert_tamper_rejection(&prog, &trace);
}

#[test]
fn rejects_tampered_lt() {
    let prog = parse_file(&get_op_path("limited_ops")).unwrap();
    let (mut trace, _) = execute(&prog).unwrap();
    trace[LT_INSTR].registers[LT_RES_REG] ^= 1;
    assert_tamper_rejection(&prog, &trace);
}
