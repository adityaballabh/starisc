use test_utils::get_op_path;
use vm::{execute, execute_with_inputs, parse_file, parse_str};

#[test]
fn exec_set() {
    let prog = parse_str("SET r8 15").unwrap();
    let (trace, regs) = execute(&prog).unwrap();
    assert_eq!(regs[8], 15);
    assert_eq!(trace.len(), 1);
    assert_eq!(trace[0].registers[8], 15);
}

#[test]
fn exec_read_inputs() {
    let prog = parse_str("READ_PRIV r8 1\nREAD_PUB r9 0\nADD r10 r8 r9").unwrap();
    let (trace, regs) = execute_with_inputs(&prog, &[11, 15], &[27]).unwrap();

    assert_eq!(regs[8], 15);
    assert_eq!(regs[9], 27);
    assert_eq!(regs[10], 42);
    assert_eq!(trace.len(), 3);
    assert_eq!(trace[0].registers[8], 15);
    assert_eq!(trace[1].registers[9], 27);
}

#[test]
fn exec_read_priv_missing_input_err() {
    let prog = parse_str("SET r3 9\nREAD_PRIV r5 1").unwrap();
    let err = execute_with_inputs(&prog, &[8], &[]).unwrap_err();

    assert_eq!(err.pc, 1);
    assert!(err.message.contains("missing private input"));
    assert!(err.message.contains("1"));
    assert_eq!(err.registers[3], 9);
}

#[test]
fn exec_read_pub_missing_input_err() {
    let prog = parse_str("SET r4 12\nREAD_PUB r5 0").unwrap();
    let err = execute_with_inputs(&prog, &[], &[]).unwrap_err();

    assert_eq!(err.pc, 1);
    assert!(err.message.contains("missing public input"));
    assert!(err.message.contains("0"));
    assert_eq!(err.registers[4], 12);
}

#[test]
fn execute_wrapper_uses_empty_inputs() {
    let prog = parse_str("READ_PRIV r5 0").unwrap();
    let err = execute(&prog).unwrap_err();

    assert_eq!(err.pc, 0);
    assert!(err.message.contains("missing private input"));
}

#[test]
fn exec_lt_true() {
    let prog = parse_str("SET r5 5\nSET r6 6\nLT r7 r5 r6").unwrap();
    let (_, regs) = execute(&prog).unwrap();
    assert_eq!(regs[7], 1);
}

#[test]
fn exec_lt_false_unique() {
    let prog = parse_str("SET r2 8\nSET r3 6\nLT r4 r2 r3").unwrap();
    let (_, regs) = execute(&prog).unwrap();
    assert_eq!(regs[4], 0);
}

#[test]
fn exec_lt_false_equal() {
    let prog = parse_str("SET r7 25\nSET r8 25\nLT r10 r7 r8").unwrap();
    let (_, regs) = execute(&prog).unwrap();
    assert_eq!(regs[10], 0);
}

#[test]
fn exec_assert_eq_pass() {
    let prog = parse_str("SET r1 15\nSET r2 15\nASSERT_EQ r1 r2").unwrap();
    let (trace, _) = execute(&prog).unwrap();
    assert_eq!(trace.len(), 3);
}

#[test]
fn exec_assert_eq_err() {
    let prog = parse_str("SET r7 52\nSET r9 34\nASSERT_EQ r7 r9").unwrap();
    let err = execute(&prog).unwrap_err();
    assert_eq!(err.pc, 2);
    assert!(err.message.contains("ASSERT_EQ failed"));
    assert_eq!(err.registers[7], 52);
    assert_eq!(err.registers[9], 34);
}

#[test]
fn exec_mod() {
    let prog = parse_str("SET r4 38\nSET r6 7\nMOD r8 r4 r6").unwrap();
    let (_, regs) = execute(&prog).unwrap();
    assert_eq!(regs[8], 3);
}

#[test]
fn exec_mod_err() {
    let prog = parse_str("SET r1 24\nMOD r3 r1 r0").unwrap();
    let err = execute(&prog).unwrap_err();
    assert_eq!(err.pc, 1);
    assert!(err.message.contains("division by 0"));
}

#[test]
fn exec_add() {
    let prog = parse_str("SET r11 33\nSET r12 17\nADD r14 r11 r12").unwrap();
    let (_, regs) = execute(&prog).unwrap();
    assert_eq!(regs[14], 50);
}

#[test]
fn exec_sub() {
    let prog = parse_str("SET r4 35\nSET r9 29\nSUB r3 r4 r9").unwrap();
    let (_, regs) = execute(&prog).unwrap();
    assert_eq!(regs[3], 6);
}

#[test]
fn exec_mul() {
    let prog = parse_str("SET r7 63\nSET r2 3\nMUL r7 r7 r2").unwrap();
    let (_, regs) = execute(&prog).unwrap();
    assert_eq!(regs[7], 189);
}

#[test]
fn exec_jz_taken_skips() {
    let prog = parse_str("SET r6 0\nJZ r6 2\nSET r3 15\nSET r4 25\nSET r5 8").unwrap();
    let (trace, regs) = execute(&prog).unwrap();
    assert_eq!(trace.len(), prog.len());
    assert_eq!(regs[3], 0);
    assert_eq!(regs[4], 0);
    assert_eq!(regs[5], 8);
    assert_eq!(trace[2].registers, trace[1].registers);
    assert_eq!(trace[3].registers, trace[1].registers);
}

#[test]
fn exec_jz_not_taken() {
    let prog = parse_str("SET r5 1\nJZ r5 2\nSET r8 8\nSET r10 12\nSET r15 20").unwrap();
    let (_, regs) = execute(&prog).unwrap();
    assert_eq!(regs[8], 8);
    assert_eq!(regs[10], 12);
    assert_eq!(regs[15], 20);
}

#[test]
fn exec_jz_to_end() {
    let prog = parse_str("JZ r0 2\nSET r7 5\nSET r9 6").unwrap();
    let (trace, regs) = execute(&prog).unwrap();
    assert_eq!(trace.len(), prog.len());
    assert_eq!(regs[7], 0);
    assert_eq!(regs[9], 0);
}

#[test]
fn exec_jz_rejects_non_bool() {
    let prog = parse_str("SET r13 6\nJZ r13 1\nSET r2 4").unwrap();
    let err = execute(&prog).unwrap_err();
    assert_eq!(err.pc, 1);
    assert!(err.message.contains("not boolean"));
}

#[test]
fn wrapping_add_overflow() {
    let prog = parse_str(&format!("SET r3 {}\nSET r5 2\nADD r4 r3 r5", u64::MAX)).unwrap();
    let (_, regs) = execute(&prog).unwrap();
    assert_eq!(regs[4], 1);
}

#[test]
fn wrapping_sub_underflow() {
    let prog = parse_str("SET r2 2\nSUB r5 r0 r2").unwrap();
    let (_, regs) = execute(&prog).unwrap();
    assert_eq!(regs[5], u64::MAX - 1);
}

#[test]
fn wrapping_mul_overflow() {
    let prog = parse_str(&format!("SET r7 {}\nSET r9 3\nMUL r8 r7 r9", u64::MAX)).unwrap();
    let (_, regs) = execute(&prog).unwrap();
    assert_eq!(regs[8], u64::MAX.wrapping_mul(3));
}

#[test]
fn trace_len_eq_prog_length() {
    let prog =
        parse_str("SET r4 8\nSET r3 9\nMUL r5 r3 r4\nMOD r5 r5 r3\nASSERT_EQ r5 r0").unwrap();
    let (trace, _) = execute(&prog).unwrap();
    assert_eq!(trace.len(), prog.len());
}

#[test]
fn trace_rows_persist() {
    let prog = parse_str("SET r8 30\nSET r9 20\nSUB r10 r8 r9").unwrap();
    let (trace, _) = execute(&prog).unwrap();
    assert_eq!(trace[0].registers[8], 30);
    assert_eq!(trace[0].registers[9], 0);
    assert_eq!(trace[1].registers[8], 30);
    assert_eq!(trace[1].registers[9], 20);
    assert_eq!(trace[2].registers[8], 30);
    assert_eq!(trace[2].registers[9], 20);
    assert_eq!(trace[2].registers[10], 10);
}

#[test]
fn sample_op_exec() {
    let prog = parse_file(&get_op_path("all_ops")).unwrap();
    let (trace, _) = execute(&prog).unwrap();
    assert_eq!(trace.len(), prog.len());
}

#[test]
fn empty_prog() {
    let (trace, regs) = execute(&[]).unwrap();
    assert!(trace.is_empty());
    assert_eq!(regs, [0; 16]);
}
