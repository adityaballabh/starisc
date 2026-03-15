use crate::instruction::Instruction;
use crate::trace::{Trace, TraceRow};
use std::fmt;

#[derive(Debug)]
pub struct ExecError {
    pub pc: usize,
    pub registers: [u64; 16],
    pub message: String,
}

impl fmt::Display for ExecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error at pc {}: {}", self.pc, self.message)
    }
}

pub fn execute(prog: &[Instruction]) -> Result<(Trace, [u64; 16]), ExecError> {
    let mut registers: [u64; 16] = [0; 16];
    let mut trace = Vec::with_capacity(prog.len());

    for (pc, instr) in prog.iter().enumerate() {
        match instr {
            Instruction::Set { dest, val } => {
                registers[*dest as usize] = *val;
            }
            Instruction::AssertEq { r1, r2 } => {
                let v1 = registers[*r1 as usize];
                let v2 = registers[*r2 as usize];
                if v1 != v2 {
                    return Err(ExecError {
                        pc,
                        registers,
                        message: format!("ASSERT_EQ failed: r{}={} != r{}={}", r1, v1, r2, v2),
                    });
                }
            }
            Instruction::Lt { dest, src1, src2 } => {
                registers[*dest as usize] = if registers[*src1 as usize] < registers[*src2 as usize]
                {
                    1
                } else {
                    0
                };
            }
            Instruction::Add { dest, src1, src2 } => {
                registers[*dest as usize] =
                    registers[*src1 as usize].wrapping_add(registers[*src2 as usize]);
            }
            Instruction::Sub { dest, src1, src2 } => {
                registers[*dest as usize] =
                    registers[*src1 as usize].wrapping_sub(registers[*src2 as usize]);
            }
            Instruction::Mul { dest, src1, src2 } => {
                registers[*dest as usize] =
                    registers[*src1 as usize].wrapping_mul(registers[*src2 as usize]);
            }
            Instruction::Mod { dest, src1, src2 } => {
                let divisor = registers[*src2 as usize];
                if divisor == 0 {
                    return Err(ExecError {
                        pc,
                        registers,
                        message: "division by 0 in MOD".to_string(),
                    });
                }
                registers[*dest as usize] = registers[*src1 as usize] % divisor;
            }
        }
        trace.push(TraceRow { registers });
    }
    Ok((trace, registers))
}
