mod common;
use vm::{execute, parse_file, parse_str};

#[test]
fn exec_set() {
    let prog = parse_str("SET r8 15").unwrap();
    let (trace, regs) = execute(&prog).unwrap();
    assert_eq!(regs[8], 15);
    assert_eq!(trace.len(), 1);
    assert_eq!(trace[0].registers[8], 15);
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
    let prog = parse_file(&common::sample_op_path()).unwrap();
    let (trace, _) = execute(&prog).unwrap();
    assert_eq!(trace.len(), prog.len());
}

#[test]
fn empty_prog() {
    let (trace, regs) = execute(&[]).unwrap();
    assert!(trace.is_empty());
    assert_eq!(regs, [0; 16]);
}
