use test_utils::get_op_path;
use vm::Instruction;
use vm::{parse_file, parse_str};

#[test]
fn parse_set() {
    let instr = parse_str("SET r10 15").unwrap();
    assert_eq!(instr, vec![Instruction::Set { dest: 10, val: 15 }]);
}

#[test]
fn parse_read_priv() {
    let instr = parse_str("READ_PRIV r10 15").unwrap();
    assert_eq!(
        instr,
        vec![Instruction::ReadPriv {
            dest: 10,
            index: 15
        }]
    );
}

#[test]
fn parse_read_pub() {
    let instr = parse_str("READ_PUB r11 2").unwrap();
    assert_eq!(instr, vec![Instruction::ReadPub { dest: 11, index: 2 }]);
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
fn parse_jz_with_offset() {
    let instr = parse_str("JZ r11 2\nSET r6 21\nSET r2 15").unwrap();
    assert_eq!(
        instr[0],
        Instruction::Jz {
            cond: 11,
            offset: 2
        }
    );
}

#[test]
fn rejects_jz_zero_offset() {
    let err = parse_str("JZ r5 0").unwrap_err();
    assert_eq!(err.line, 1);
    assert!(err.message.contains("greater than 0"));
}

#[test]
fn rejects_jz_invalid_offset() {
    let err = parse_str("JZ r4 -3").unwrap_err();
    assert_eq!(err.line, 1);
    assert!(err.message.contains("offset"));
}

#[test]
fn rejects_jz_out_of_bounds() {
    let err = parse_str("JZ r7 5\nSET r9 3").unwrap_err();
    assert_eq!(err.line, 1);
    assert!(err.message.contains("past program end"));
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
fn rejects_read_priv_r0_write() {
    let err = parse_str("READ_PRIV r0 4").unwrap_err();
    assert_eq!(err.line, 1);
    assert!(err.message.contains("r0"));
}

#[test]
fn rejects_read_pub_r0_write() {
    let err = parse_str("READ_PUB r0 4").unwrap_err();
    assert_eq!(err.line, 1);
    assert!(err.message.contains("r0"));
}

#[test]
fn rejects_read_priv_negative_index() {
    let err = parse_str("READ_PRIV r1 -4").unwrap_err();
    assert_eq!(err.line, 1);
    assert!(err.message.contains("input index"));
}

#[test]
fn rejects_read_pub_non_int_index() {
    let err = parse_str("READ_PUB r1 nope").unwrap_err();
    assert_eq!(err.line, 1);
    assert!(err.message.contains("nope"));
}

#[test]
fn rejects_set_with_non_int() {
    let err = parse_str("SET r3 de34").unwrap_err();
    assert_eq!(err.line, 1);
    assert!(err.message.contains("de34"));
}

#[test]
fn skips_comments_and_blank_lines() {
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
    parse_file(&get_op_path("all_ops")).unwrap();
}

macro_rules! has_instr {
    ($prog:expr, $instr:path) => {
        assert!($prog.iter().any(|i| matches!(i, $instr { .. })))
    };
}

#[test]
fn sample_op_covers_arithmetic_and_assert_instrs() {
    let prog = parse_file(&get_op_path("all_ops")).unwrap();
    has_instr!(prog, Instruction::Set);
    has_instr!(prog, Instruction::Add);
    has_instr!(prog, Instruction::Sub);
    has_instr!(prog, Instruction::Mul);
    has_instr!(prog, Instruction::Mod);
    has_instr!(prog, Instruction::AssertEq);
    has_instr!(prog, Instruction::Lt);
}
