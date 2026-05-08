use std::fmt;

pub const OP_SET: &str = "SET";
pub const OP_READ_PRIV: &str = "READ_PRIV";
pub const OP_READ_PUB: &str = "READ_PUB";
pub const OP_ADD: &str = "ADD";
pub const OP_SUB: &str = "SUB";
pub const OP_MUL: &str = "MUL";
pub const OP_MOD: &str = "MOD";
pub const OP_ASSERT_EQ: &str = "ASSERT_EQ";
pub const OP_LT: &str = "LT";
pub const OP_JZ: &str = "JZ";

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    Set { dest: u8, val: u64 },
    ReadPriv { dest: u8, index: usize },
    ReadPub { dest: u8, index: usize },
    Add { dest: u8, src1: u8, src2: u8 },
    Sub { dest: u8, src1: u8, src2: u8 },
    Mul { dest: u8, src1: u8, src2: u8 },
    Mod { dest: u8, src1: u8, src2: u8 },
    AssertEq { r1: u8, r2: u8 },
    Lt { dest: u8, src1: u8, src2: u8 },
    Jz { cond: u8, offset: usize },
}

fn fmt_arith(f: &mut fmt::Formatter<'_>, op: &str, dest: &u8, src1: &u8, src2: &u8) -> fmt::Result {
    write!(f, "{:<9} r{} r{} r{}", op, dest, src1, src2)
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::Set { dest, val } => write!(f, "{:<9} r{} {}", OP_SET, dest, val),
            Instruction::ReadPriv { dest, index } => {
                write!(f, "{:<9} r{} {}", OP_READ_PRIV, dest, index)
            }
            Instruction::ReadPub { dest, index } => {
                write!(f, "{:<9} r{} {}", OP_READ_PUB, dest, index)
            }
            Instruction::AssertEq { r1, r2 } => write!(f, "{:<9} r{} r{}", OP_ASSERT_EQ, r1, r2),
            Instruction::Add { dest, src1, src2 } => fmt_arith(f, OP_ADD, dest, src1, src2),
            Instruction::Sub { dest, src1, src2 } => fmt_arith(f, OP_SUB, dest, src1, src2),
            Instruction::Mul { dest, src1, src2 } => fmt_arith(f, OP_MUL, dest, src1, src2),
            Instruction::Mod { dest, src1, src2 } => fmt_arith(f, OP_MOD, dest, src1, src2),
            Instruction::Lt { dest, src1, src2 } => fmt_arith(f, OP_LT, dest, src1, src2),
            Instruction::Jz { cond, offset } => write!(f, "{:<9} r{} {}", OP_JZ, cond, offset),
        }
    }
}
