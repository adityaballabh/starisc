mod common;
use vm::Instruction;
use vm::{parse_file, parse_str};

#[test]
fn parse_set() {
    let instr = parse_str("SET r10 15").unwrap();
    assert_eq!(instr, vec![Instruction::Set { dest: 10, val: 15 }]);
}

#[test]
fn parse_add() {
    let instr = parse_str("ADD r8 r6 r7").unwrap();
    assert_eq!(
        instr,
        vec![Instruction::Add {
            dest: 8,
            src1: 6,
            src2: 7
        }]
    );
}

#[test]
fn parse_sub() {
    let instr = parse_str("SUB r12 r10 r11").unwrap();
    assert_eq!(
        instr,
        vec![Instruction::Sub {
            dest: 12,
            src1: 10,
            src2: 11
        }]
    );
}

#[test]
fn parse_mul() {
    let instr = parse_str("MUL r9 r10 r11").unwrap();
    assert_eq!(
        instr,
        vec![Instruction::Mul {
            dest: 9,
            src1: 10,
            src2: 11
        }]
    );
}

#[test]
fn parse_mod() {
    let instr = parse_str("MOD r12 r13 r14").unwrap();
    assert_eq!(
        instr,
        vec![Instruction::Mod {
            dest: 12,
            src1: 13,
            src2: 14
        }]
    );
}

#[test]
fn parse_assert_eq() {
    let instr = parse_str("ASSERT_EQ r8 r15").unwrap();
    assert_eq!(instr, vec![Instruction::AssertEq { r1: 8, r2: 15 }]);
}

#[test]
fn parse_lt() {
    let instr = parse_str("LT r4 r7 r8").unwrap();
    assert_eq!(
        instr,
        vec![Instruction::Lt {
            dest: 4,
            src1: 7,
            src2: 8
        }]
    );
}

#[test]
fn rejects_unknown_opcode() {
    let err = parse_str("UNK r8 r9").unwrap_err();
    assert_eq!(err.line, 1);
    assert!(err.message.contains("UNK"));
}

#[test]
fn rejects_invalid_register() {
    let err = parse_str("ADD r5 r16 r3").unwrap_err();
    assert_eq!(err.line, 1);
    assert!(err.message.contains("r16"));
}

#[test]
fn rejects_r0_write() {
    let err = parse_str("SET r0 4").unwrap_err();
    assert_eq!(err.line, 1);
    assert!(err.message.contains("r0"));
}

#[test]
fn rejects_set_with_non_int() {
    let err = parse_str("SET r3 de34").unwrap_err();
    assert_eq!(err.line, 1);
    assert!(err.message.contains("de34"));
}

#[test]
fn skips_non_op() {
    let prog = parse_str("# should be skipped\n\nSET r1 1\n").unwrap();
    assert_eq!(prog.len(), 1);
}

#[test]
fn accepts_r0_source() {
    let instr = parse_str("ADD r10 r8 r0").unwrap();
    assert_eq!(
        instr,
        vec![Instruction::Add {
            dest: 10,
            src1: 8,
            src2: 0
        }]
    );
}

#[test]
fn error_with_correct_line() {
    let err = parse_str("SET r5 4\nUNK r8 r9").unwrap_err();
    assert_eq!(err.line, 2);
}

#[test]
fn parse_sample_op_succeeds() {
    parse_file(&common::sample_op_path()).unwrap();
}

macro_rules! has_instr {
    ($prog:expr, $instr:path) => {
        assert!($prog.iter().any(|i| matches!(i, $instr { .. })))
    };
}

#[test]
fn sample_op_covers_all_instr() {
    let prog = parse_file(&common::sample_op_path()).unwrap();
    has_instr!(prog, Instruction::Set);
    has_instr!(prog, Instruction::Add);
    has_instr!(prog, Instruction::Sub);
    has_instr!(prog, Instruction::Mul);
    has_instr!(prog, Instruction::Mod);
    has_instr!(prog, Instruction::AssertEq);
    has_instr!(prog, Instruction::Lt);
}
