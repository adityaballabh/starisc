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
