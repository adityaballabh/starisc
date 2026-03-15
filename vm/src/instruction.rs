use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    Set { dest: u8, val: u64 },
    Add { dest: u8, src1: u8, src2: u8 },
    Sub { dest: u8, src1: u8, src2: u8 },
    Mul { dest: u8, src1: u8, src2: u8 },
    Mod { dest: u8, src1: u8, src2: u8 },
    AssertEq { r1: u8, r2: u8 },
    Lt { dest: u8, src1: u8, src2: u8 },
}

fn fmt_arith(f: &mut fmt::Formatter<'_>, op: &str, dest: &u8, src1: &u8, src2: &u8) -> fmt::Result {
    write!(f, "{:<9} r{} r{} r{}", op, dest, src1, src2)
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::Set { dest, val } => write!(f, "{:<9} r{} {}", "SET", dest, val),
            Instruction::AssertEq { r1, r2 } => write!(f, "{:<9} r{} r{}", "ASSERT_EQ", r1, r2),
            Instruction::Add { dest, src1, src2 } => fmt_arith(f, "ADD", dest, src1, src2),
            Instruction::Sub { dest, src1, src2 } => fmt_arith(f, "SUB", dest, src1, src2),
            Instruction::Mul { dest, src1, src2 } => fmt_arith(f, "MUL", dest, src1, src2),
            Instruction::Mod { dest, src1, src2 } => fmt_arith(f, "MOD", dest, src1, src2),
            Instruction::Lt { dest, src1, src2 } => fmt_arith(f, "LT", dest, src1, src2),
        }
    }
}
