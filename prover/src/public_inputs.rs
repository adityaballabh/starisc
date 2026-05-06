use crate::Felt;
use vm::Instruction;
use winterfell::math::{FieldElement, ToElements};

pub const NUM_PERIODIC_COLS: usize = 61;

pub const P_IS_SET: usize = 0;
pub const P_IS_ADD: usize = 1;
pub const P_IS_SUB: usize = 2;
pub const P_IS_MUL: usize = 3;
pub const P_IS_ASSERT_EQ: usize = 4;
pub const P_IS_LT: usize = 5;
pub const P_IS_MOD: usize = 6;
pub const P_IS_NOP: usize = 7;
pub const P_IS_JZ: usize = 8;
pub const P_IS_READ_PRIV: usize = 9;
pub const P_IS_READ_PUB: usize = 10;
// one-hot register selectors for res, src1, src2
pub const P_RES_BASE: usize = 11;
pub const P_SRC1_BASE: usize = 27;
pub const P_SRC2_BASE: usize = 43;
pub const P_CONST: usize = 59;
pub const P_OFFSET: usize = 60;

#[derive(Clone, Debug)]
pub struct PublicInputs {
    pub prog: Vec<Instruction>,
    pub public_inputs: Vec<u64>,
    pub trace_len: usize,
    // precomputed flags to set constraint degrees
    pub dest_mask: [bool; 16], // true if reg used as dest
    pub bits_used: u64, // bitmask. set to 1 if the bit is used in any row (lt/mod diff or value)
    pub wrap_bits_used: u64, // bitmask. set to 1 if the bit is used in any wrapping witness row
    pub has_nonzero_src1: bool,
    pub has_nonzero_src2: bool,
    pub has_mul: bool,
    pub has_assert_eq: bool,
    pub has_lt: bool,
    pub has_mod: bool,
    pub has_jz: bool,
    pub has_taken_jz: bool,
    pub has_not_taken_jz: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct PublicInputFlags {
    pub has_nonzero_src1: bool,
    pub has_nonzero_src2: bool,
    pub has_taken_jz: bool,
    pub has_not_taken_jz: bool,
}

fn set_selectors(
    cols: &mut [Vec<Felt>],
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
    pub fn new(
        prog: Vec<Instruction>,
        public_inputs: Vec<u64>,
        trace_len: usize,
        bits_used: u64,
        wrap_bits_used: u64,
        flags: PublicInputFlags,
    ) -> Self {
        let mut dest_mask = [false; 16];
        let mut has_mul = false;
        let mut has_assert_eq = false;
        let mut has_lt = false;
        let mut has_mod = false;
        let mut has_jz = false;
        for instr in &prog {
            collect_instr_shape(
                instr,
                &mut dest_mask,
                &mut has_mul,
                &mut has_assert_eq,
                &mut has_lt,
                &mut has_mod,
                &mut has_jz,
            );
        }
        Self {
            prog,
            public_inputs,
            trace_len,
            dest_mask,
            bits_used,
            wrap_bits_used,
            has_nonzero_src1: flags.has_nonzero_src1,
            has_nonzero_src2: flags.has_nonzero_src2,
            has_mul,
            has_assert_eq,
            has_lt,
            has_mod,
            has_jz,
            has_taken_jz: flags.has_taken_jz,
            has_not_taken_jz: flags.has_not_taken_jz,
        }
    }

    pub fn build_periodic_columns(&self) -> Vec<Vec<Felt>> {
        let n = self.trace_len;
        let mut cols = vec![vec![Felt::ZERO; n]; NUM_PERIODIC_COLS];

        for (i, instr) in self.prog.iter().enumerate() {
            set_periodic_instr(&mut cols, i, instr, &self.public_inputs);
        }
        let pad_start = self.prog.len();
        if pad_start < n {
            cols[P_IS_NOP][pad_start..n].fill(Felt::ONE);
            cols[P_SRC1_BASE][pad_start..n].fill(Felt::ONE);
            cols[P_SRC2_BASE][pad_start..n].fill(Felt::ONE);
        }
        cols
    }

    pub fn has_result_constraint(&self) -> bool {
        self.prog.iter().any(|instr| {
            matches!(
                instr,
                Instruction::Set { .. }
                    | Instruction::ReadPriv { .. }
                    | Instruction::ReadPub { .. }
                    | Instruction::Add { .. }
                    | Instruction::Sub { .. }
                    | Instruction::Mul { .. }
                    | Instruction::AssertEq { .. }
                    | Instruction::Jz { .. }
            )
        })
    }
}

fn collect_instr_shape(
    instr: &Instruction,
    dest_mask: &mut [bool; 16],
    has_mul: &mut bool,
    has_assert_eq: &mut bool,
    has_lt: &mut bool,
    has_mod: &mut bool,
    has_jz: &mut bool,
) {
    match instr {
        Instruction::Set { dest, .. }
        | Instruction::ReadPriv { dest, .. }
        | Instruction::ReadPub { dest, .. }
        | Instruction::Add { dest, .. }
        | Instruction::Sub { dest, .. } => {
            dest_mask[*dest as usize] = true;
        }
        Instruction::Mod { dest, .. } => {
            dest_mask[*dest as usize] = true;
            *has_mod = true;
        }
        Instruction::Mul { dest, .. } => {
            dest_mask[*dest as usize] = true;
            *has_mul = true;
        }
        Instruction::AssertEq { .. } => {
            *has_assert_eq = true;
        }
        Instruction::Lt { dest, .. } => {
            dest_mask[*dest as usize] = true;
            *has_lt = true;
        }
        Instruction::Jz { .. } => {
            *has_jz = true;
        }
    }
}

fn set_periodic_instr(
    cols: &mut [Vec<Felt>],
    i: usize,
    instr: &Instruction,
    public_inputs: &[u64],
) {
    match instr {
        Instruction::Set { dest, val } => {
            set_selectors(cols, i, P_IS_SET, Some(*dest), 0, 0, Some(*val))
        }
        Instruction::ReadPriv { dest, .. } => {
            set_selectors(cols, i, P_IS_READ_PRIV, Some(*dest), 0, 0, None)
        }
        Instruction::ReadPub { dest, index } => {
            let value = public_inputs.get(*index).copied().unwrap_or(0);
            set_selectors(cols, i, P_IS_READ_PUB, Some(*dest), 0, 0, Some(value))
        }
        Instruction::Add { dest, src1, src2 } => {
            set_selectors(cols, i, P_IS_ADD, Some(*dest), *src1, *src2, None)
        }
        Instruction::Sub { dest, src1, src2 } => {
            set_selectors(cols, i, P_IS_SUB, Some(*dest), *src1, *src2, None)
        }
        Instruction::Mul { dest, src1, src2 } => {
            set_selectors(cols, i, P_IS_MUL, Some(*dest), *src1, *src2, None)
        }
        Instruction::AssertEq { r1, r2 } => {
            set_selectors(cols, i, P_IS_ASSERT_EQ, None, *r1, *r2, None)
        }
        Instruction::Lt { dest, src1, src2 } => {
            set_selectors(cols, i, P_IS_LT, Some(*dest), *src1, *src2, None)
        }
        Instruction::Mod { dest, src1, src2 } => {
            set_selectors(cols, i, P_IS_MOD, Some(*dest), *src1, *src2, None)
        }
        Instruction::Jz { cond, offset } => {
            set_selectors(cols, i, P_IS_JZ, None, *cond, 0, None);
            cols[P_OFFSET][i] = Felt::from(*offset as u64);
        }
    }
}

impl ToElements<Felt> for PublicInputs {
    fn to_elements(&self) -> Vec<Felt> {
        let mut elements = Vec::new();
        elements.push(Felt::from(self.prog.len() as u64));
        for instr in &self.prog {
            let (opcode, dest, src1, src2, val) = match instr {
                Instruction::Set { dest, val } => (P_IS_SET, Some(*dest), None, None, Some(*val)),
                Instruction::ReadPriv { dest, index } => {
                    (P_IS_READ_PRIV, Some(*dest), None, None, Some(*index as u64))
                }
                Instruction::ReadPub { dest, index } => {
                    (P_IS_READ_PUB, Some(*dest), None, None, Some(*index as u64))
                }
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
                Instruction::Lt { dest, src1, src2 } => {
                    (P_IS_LT, Some(*dest), Some(*src1), Some(*src2), None)
                }
                Instruction::Mod { dest, src1, src2 } => {
                    (P_IS_MOD, Some(*dest), Some(*src1), Some(*src2), None)
                }
                Instruction::Jz { cond, offset } => {
                    (P_IS_JZ, None, Some(*cond), None, Some(*offset as u64))
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
        elements.push(Felt::from(self.public_inputs.len() as u64));
        for value in &self.public_inputs {
            elements.push(Felt::from(*value));
        }
        elements.push(Felt::from(self.trace_len as u64));
        elements
    }
}
