use prover::prover::{prove, verify};
use test_utils::{assert_proof_rejected, assert_proof_accepted, get_op_path};
use vm::{execute, parse_file, parse_str, Instruction, Trace};

fn parse_program(src: &str) -> Vec<Instruction> {
    parse_str(src).unwrap()
}

fn load_program(name: &str) -> Vec<Instruction> {
    parse_file(&get_op_path(name)).unwrap()
}

fn exec_trace(prog: &[Instruction]) -> Trace {
    execute(prog).unwrap().0
}

fn assert_prove_verify(prog: &[Instruction]) {
    let trace = exec_trace(prog);
    assert!(verify(prog, prove(prog, &trace).unwrap()).is_ok());
}

fn assert_program_proves(src: &str) {
    let prog = parse_program(src);
    assert_prove_verify(&prog);
}

fn assert_tamper_rejection(prog: &[Instruction], trace: &Trace) {
    let (trace_clone, prog_vec) = (trace.clone(), prog.to_vec());
    assert_proof_rejected(
        move || prove(&prog_vec, &trace_clone),
        |proof| verify(prog, proof),
    );
}

fn assert_tamper_accepted(prog: &[Instruction], trace: &Trace) {
    let (trace_clone, prog_vec) = (trace.clone(), prog.to_vec());
    assert_proof_accepted(
        move || prove(&prog_vec, &trace_clone),
        |proof| verify(prog, proof),
    );
}

#[test]
fn prove_verify_set() {
    assert_program_proves(
        "SET r1 42
         SET r2 42
         ADD r3 r1 r0
         ASSERT_EQ r3 r2",
    );
}

#[test]
fn prove_verify_add() {
    assert_program_proves(
        "SET r1 7
         SET r2 9
         ADD r3 r1 r2",
    );
}

#[test]
fn prove_verify_sub() {
    assert_program_proves(
        "SET r1 9
         SET r2 4
         SUB r3 r1 r2",
    );
}

#[test]
fn prove_verify_mul() {
    assert_program_proves(
        "SET r1 7
         SET r2 9
         MUL r3 r1 r2",
    );
}

#[test]
fn prove_verify_mod() {
    assert_program_proves(
        "SET r1 55
         SET r2 15
         MOD r3 r1 r2
         SET r4 10
         ASSERT_EQ r3 r4",
    );
}

#[test]
fn prove_verify_lt() {
    assert_program_proves(
        "SET r1 7
         SET r2 9
         LT r3 r1 r2",
    );
}

#[test]
fn prove_verify_assert_eq() {
    assert_program_proves(
        "SET r1 40
         SET r2 2
         ADD r3 r1 r2
         SET r4 42
         ASSERT_EQ r3 r4",
    );
}

#[test]
fn prove_verify_all_ops() {
    let prog = load_program("all_ops");
    assert_prove_verify(&prog);
}

#[test]
fn prove_verify_limited_ops() {
    let prog = load_program("limited_ops");
    assert_prove_verify(&prog);
}

#[test]
fn prove_verify_no_mul() {
    assert_program_proves(
        "SET r1 21
         SET r2 8
         ADD r3 r1 r2
         SUB r4 r3 r2
         SET r5 21
         ASSERT_EQ r4 r5
         MOD r6 r3 r2
         SET r7 5
         ASSERT_EQ r6 r7
         LT r8 r4 r3
         SET r9 1
         ASSERT_EQ r8 r9",
    );
}

#[test]
fn prove_verify_no_lt() {
    assert_program_proves(
        "SET r1 8
         SET r2 5
         ADD r3 r1 r2
         MUL r4 r3 r2
         SUB r5 r4 r1
         MOD r6 r5 r2
         SET r7 2
         ASSERT_EQ r6 r7",
    );
}

#[test]
fn prove_verify_empty_program() {
    let prog = parse_program("");
    assert_prove_verify(&prog);
}

#[test]
fn prove_verify_add_overflow() {
    assert_program_proves(&format!("SET r1 {}\nSET r2 1\nADD r3 r1 r2", u64::MAX));
}

#[test]
fn prove_verify_add_overflow_multiple() {
    assert_program_proves(&format!("SET r1 {}\nSET r2 1\nADD r3 r1 r2\nADD r3 r3 r1\nADD r3 r3 r1\nSET r4 18446744073709551614\nASSERT_EQ r3 r4", u64::MAX));
}

#[test]
fn prove_verify_sub_underflow() {
    assert_program_proves("SET r1 0\nSET r2 1\nSUB r3 r1 r2");
}

#[test]
fn prove_verify_mul_overflow() {
    assert_program_proves(&format!(
        "SET r1 {}\nSET r2 2\nMUL r3 r1 r2",
        (u64::MAX / 2) + 1
    ));
}

#[test]
fn lt_equal_unequal_values() {
    assert_program_proves(
        "SET r1 17
         SET r2 17
         LT r3 r1 r2
         SET r4 0
         ASSERT_EQ r3 r4
         SET r5 18
         LT r6 r1 r5
         SET r7 1
         ASSERT_EQ r6 r7",
    );
}

#[test]
fn lt_with_max_u64() {
    assert_program_proves(&format!(
        "SET r1 {}\nSET r2 {}\nLT r3 r1 r2\nSET r4 1\nASSERT_EQ r3 r4",
        u64::MAX - 1,
        u64::MAX
    ));
}


////////////////////////////////////
/* MOD TEST                       */

#[test]
fn mod_result_eq_0_1() {
    assert_program_proves(
        "SET r1 3
         SET r2 3
         MOD r3 r1 r2
         SET r4 0
         ASSERT_EQ r3 r4",
    );
}

#[test]
fn mod_result_happy_path() {
    assert_program_proves(
        "SET r1 4
         SET r2 3
         MOD r3 r1 r2
         SET r4 1
         ASSERT_EQ r3 r4",
    );
}

#[test]
fn mod_result_same() {
    assert_program_proves(
        "SET r1 3
         SET r2 4
         MOD r3 r1 r2
         SET r4 3
         ASSERT_EQ r3 r4",
    );
}

#[test]
fn mod_result_eq_0_2() {
    assert_program_proves(
        "SET r1 13137
         SET r2 1
         MOD r3 r1 r2
         SET r4 0
         ASSERT_EQ r3 r4",
    );
}


////////////////////////////////////

////////////////////////////////////
/* this simply modifies the trace */
#[test]
fn rejects_tampered_register() {
    let prog = parse_program("SET r1 42");
    let mut trace = exec_trace(&prog);
    trace[0].registers[1] += 1337;
    assert_tamper_rejection(&prog, &trace);
}

#[test]
fn rejects_tampered_lt_result() {
    let prog = parse_program(
        "SET r1 10
         SET r2 20
         LT r3 r1 r2",
    );
    let mut trace = exec_trace(&prog);
    trace[2].registers[3] ^= 1;
    assert_tamper_rejection(&prog, &trace);
    trace[2].registers[3] ^= 1;
    assert_tamper_accepted(&prog, &trace);
}

#[test]
fn rejects_tampered_false_assert_eq() {
    let prog = parse_program(
        "SET r1 4
         SET r2 4
         ASSERT_EQ r1 r2",
    );
    let mut trace = exec_trace(&prog);
    trace[2].registers[2] = 5;
    assert_tamper_rejection(&prog, &trace);
}

////////////////////////////////////

////////////////////////////////////
/* error test case                */
#[test]
fn exec_assert_eq_fail() {
    let prog = parse_program(
        "SET r1 4
         SET r2 5
         ASSERT_EQ r1 r2",
    );
    let err = execute(&prog).unwrap_err();
    assert_eq!(err.pc, 2);
    assert!(err.message.contains("ASSERT_EQ failed"));
}

#[test]
fn exec_mod_div_by_zero() {
    let prog = parse_program(
        "SET r1 4
         SET r2 0
         MOD r3 r1 r2",
    );
    let err = execute(&prog).unwrap_err();
    assert_eq!(err.pc, 2);
    assert!(err.message.contains("division by 0 in MOD"));
}
////////////////////////////////////
