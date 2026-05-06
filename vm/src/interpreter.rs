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

pub fn execute(prog: &[Instruction]) -> Result<(Trace, [u64; 16]), Box<ExecError>> {
    let mut registers: [u64; 16] = [0; 16];
    let mut trace = Vec::with_capacity(prog.len());
    let mut pc = 0;

    while pc < prog.len() {
        let instr = &prog[pc];
        match instr {
            Instruction::Set { dest, val } => {
                registers[*dest as usize] = *val;
            }
            Instruction::AssertEq { r1, r2 } => {
                let v1 = registers[*r1 as usize];
                let v2 = registers[*r2 as usize];
                if v1 != v2 {
                    return Err(Box::new(ExecError {
                        pc,
                        registers,
                        message: format!("ASSERT_EQ failed: r{}={} != r{}={}", r1, v1, r2, v2),
                    }));
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
                    return Err(Box::new(ExecError {
                        pc,
                        registers,
                        message: "division by 0 in MOD".to_string(),
                    }));
                }
                registers[*dest as usize] = registers[*src1 as usize] % divisor;
            }
            Instruction::Jz { cond, offset } => {
                let cond_val = registers[*cond as usize];
                if cond_val != 0 && cond_val != 1 {
                    return Err(Box::new(ExecError {
                        pc,
                        registers,
                        message: format!("JZ condition r{}={} is not boolean", cond, cond_val),
                    }));
                }
                let target = pc + 1 + offset;
                if target > prog.len() {
                    return Err(Box::new(ExecError {
                        pc,
                        registers,
                        message: format!("JZ target {} is past program end {}", target, prog.len()),
                    }));
                }
                trace.push(TraceRow { registers });
                if cond_val == 0 {
                    for _ in (pc + 1)..target {
                        trace.push(TraceRow { registers });
                    }
                    pc = target;
                } else {
                    pc += 1;
                }
                continue;
            }
        }
        trace.push(TraceRow { registers });
        pc += 1;
    }
    Ok((trace, registers))
}
