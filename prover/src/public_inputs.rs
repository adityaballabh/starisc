use crate::Felt;
use vm::Instruction;
use winterfell::math::{FieldElement, ToElements};

pub const NUM_PERIODIC_COLS: usize = 57;

pub const P_IS_SET: usize = 0;
pub const P_IS_ADD: usize = 1;
pub const P_IS_SUB: usize = 2;
pub const P_IS_MUL: usize = 3;
pub const P_IS_ASSERT_EQ: usize = 4;
pub const P_IS_MOD: usize = 5;
pub const P_IS_LT: usize = 6;
pub const P_IS_NOP: usize = 7;
// one-hot register selectors for res, src1, src2
pub const P_RES_BASE: usize = 8;
pub const P_SRC1_BASE: usize = 24;
pub const P_SRC2_BASE: usize = 40;
pub const P_CONST: usize = 56;

#[derive(Clone, Debug)]
pub struct PublicInputs {
    pub prog: Vec<Instruction>,
    pub trace_len: usize,
}

fn set_selectors(
    cols: &mut Vec<Vec<Felt>>,
    i: usize,
    flag: usize,
    dest: Option<u8>,
    src1: u8,
    src2: u8,
    const_val: Option<u64>,
) {
    cols[flag][i] = Felt::ONE;
    if let Some(d) = dest {
        cols[P_RES_BASE + d as usize][i] = Felt::ONE;
    }
    cols[P_SRC1_BASE + src1 as usize][i] = Felt::ONE;
    cols[P_SRC2_BASE + src2 as usize][i] = Felt::ONE;
    if let Some(v) = const_val {
        cols[P_CONST][i] = Felt::from(v);
    }
}

impl PublicInputs {
    pub fn new(prog: Vec<Instruction>, trace_len: usize) -> Self {
        Self { prog, trace_len }
    }

    pub fn build_periodic_columns(&self) -> Vec<Vec<Felt>> {
        let n = self.trace_len;
        let mut cols = vec![vec![Felt::ZERO; n]; NUM_PERIODIC_COLS];

        for (i, instr) in self.prog.iter().enumerate() {
            match instr {
                Instruction::Set { dest, val } => {
                    set_selectors(&mut cols, i, P_IS_SET, Some(*dest), 0, 0, Some(*val))
                }
                Instruction::Add { dest, src1, src2 } => {
                    set_selectors(&mut cols, i, P_IS_ADD, Some(*dest), *src1, *src2, None)
                }
                Instruction::Sub { dest, src1, src2 } => {
                    set_selectors(&mut cols, i, P_IS_SUB, Some(*dest), *src1, *src2, None)
                }
                Instruction::Mul { dest, src1, src2 } => {
                    set_selectors(&mut cols, i, P_IS_MUL, Some(*dest), *src1, *src2, None)
                }
                Instruction::AssertEq { r1, r2 } => {
                    set_selectors(&mut cols, i, P_IS_ASSERT_EQ, None, *r1, *r2, None)
                }
                Instruction::Mod { dest, src1, src2 } => {
                    set_selectors(&mut cols, i, P_IS_MOD, Some(*dest), *src1, *src2, None)
                }
                Instruction::Lt { dest, src1, src2 } => {
                    set_selectors(&mut cols, i, P_IS_LT, Some(*dest), *src1, *src2, None)
                }
            }
        }
        // NOP padding
        for i in self.prog.len()..n {
            cols[P_IS_NOP][i] = Felt::ONE;
            cols[P_SRC1_BASE][i] = Felt::ONE; // point to r0
            cols[P_SRC2_BASE][i] = Felt::ONE;
        }
        cols
    }
}

impl ToElements<Felt> for PublicInputs {
    fn to_elements(&self) -> Vec<Felt> {
        let mut elements = Vec::new();
        elements.push(Felt::from(self.prog.len() as u64));
        for instr in &self.prog {
            let (opcode, dest, src1, src2, val) = match instr {
                Instruction::Set { dest, val } => (P_IS_SET, Some(*dest), None, None, Some(*val)),
                Instruction::Add { dest, src1, src2 } => {
                    (P_IS_ADD, Some(*dest), Some(*src1), Some(*src2), None)
                }
                Instruction::Sub { dest, src1, src2 } => {
                    (P_IS_SUB, Some(*dest), Some(*src1), Some(*src2), None)
                }
                Instruction::Mul { dest, src1, src2 } => {
                    (P_IS_MUL, Some(*dest), Some(*src1), Some(*src2), None)
                }
                Instruction::AssertEq { r1, r2 } => {
                    (P_IS_ASSERT_EQ, None, Some(*r1), Some(*r2), None)
                }
                Instruction::Mod { dest, src1, src2 } => {
                    (P_IS_MOD, Some(*dest), Some(*src1), Some(*src2), None)
                }
                Instruction::Lt { dest, src1, src2 } => {
                    (P_IS_LT, Some(*dest), Some(*src1), Some(*src2), None)
                }
            };
            elements.push(Felt::from(opcode as u64));
            for reg in [dest, src1, src2].into_iter().flatten() {
                elements.push(Felt::from(reg as u64));
            }
            if let Some(const_val) = val {
                elements.push(Felt::from(const_val));
            }
        }
        elements.push(Felt::from(self.trace_len as u64));
        elements
    }
}
