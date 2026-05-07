use crate::{
    Felt, ACTIVE_COL, COND_COL, NUM_RANGE_BITS, NUM_REGISTERS, QUOT_COL, RANGE_BITS_BASE, RES_COL,
    SKIP_COUNTDOWN_COL, SKIP_COUNTDOWN_INV_COL, SRC1_COL, SRC2_COL, TRACE_WIDTH, WRAP_BITS_BASE,
};
use std::array::from_fn;
use vm::{Instruction, Trace};
use winterfell::math::{FieldElement, StarkField};
use winterfell::TraceTable;

pub fn get_trace_len(prog: &[Instruction]) -> usize {
    // +1 for initial row. winterfell restriction: min 8 and power of 2
    (prog.len() + 1).next_power_of_two().max(32)
}

fn get_ops(regs: &[u64; 16], s1: u8, s2: u8) -> (u64, u64) {
    (regs[s1 as usize], regs[s2 as usize])
}

fn perform_binary_op(regs: &[u64; 16], s1: u8, s2: u8, op: fn(u64, u64) -> u64) -> (u64, u64, u64) {
    let (a, b) = get_ops(regs, s1, s2);
    (a, b, op(a, b))
}

fn add_wrap(a: u64, b: u64) -> u64 {
    ((a as u128 + b as u128) >> 64) as u64
}

fn sub_wrap(a: u64, b: u64) -> u64 {
    (a < b) as u64
}

fn mul_wrap(a: u64, b: u64) -> u64 {
    ((a as u128 * b as u128) >> 64) as u64
}

pub fn build_trace(prog: &[Instruction], vm_trace: &Trace) -> TraceTable<Felt> {
    assert_eq!(prog.len(), vm_trace.len());
    let n = prog.len();
    let trace_len = get_trace_len(prog);
    let mut cols = vec![vec![Felt::ZERO; trace_len]; TRACE_WIDTH];
    let mut skip_countdown = 0usize;

    for (i, (instr, row)) in prog.iter().zip(vm_trace.iter()).enumerate() {
        let out_row = i + 1;
        let active = skip_countdown == 0;
        cols[ACTIVE_COL][i] = Felt::from(active as u8);
        cols[SKIP_COUNTDOWN_COL][i] = Felt::from(skip_countdown as u64);
        cols[SKIP_COUNTDOWN_INV_COL][i] = if skip_countdown == 0 {
            Felt::ZERO
        } else {
            Felt::from(skip_countdown as u64).inv()
        };

        for (r, col) in cols.iter_mut().enumerate().take(NUM_REGISTERS) {
            col[out_row] = Felt::from(row.registers[r]);
        }

        let prev_regs: [u64; 16] = from_fn(|r| cols[r][out_row - 1].as_int() as u64);

        let (s1, s2, mut res, mut quot, mut wrap) = match instr {
            Instruction::Set { val, .. } => (0, 0, *val, 1, 0),
            Instruction::ReadPriv { dest, .. } | Instruction::ReadPub { dest, .. } => {
                (0, 0, row.registers[*dest as usize], 1, 0)
            }
            Instruction::Add { src1, src2, .. } => {
                let (s1, s2, res) = perform_binary_op(&prev_regs, *src1, *src2, u64::wrapping_add);
                (s1, s2, res, 1, add_wrap(s1, s2))
            }
            Instruction::Sub { src1, src2, .. } => {
                let (s1, s2, res) = perform_binary_op(&prev_regs, *src1, *src2, u64::wrapping_sub);
                (s1, s2, res, 1, sub_wrap(s1, s2))
            }
            Instruction::Mul { src1, src2, .. } => {
                let (s1, s2, res) = perform_binary_op(&prev_regs, *src1, *src2, u64::wrapping_mul);
                (s1, s2, res, 1, mul_wrap(s1, s2))
            }
            Instruction::AssertEq { r1, r2 } => {
                let (a, b) = get_ops(&prev_regs, *r1, *r2);
                // Store 1 on ASSERT_EQ rows so the equality constraint keeps a stable degree.
                (a, b, 1, 1, 0)
            }
            Instruction::Mod { src1, src2, .. } => {
                let (a, b) = get_ops(&prev_regs, *src1, *src2);
                // Store quotient + 1 so the MOD quotient witness is never identically zero.
                if active {
                    (a, b, a % b, (a / b) + 1, 0)
                } else {
                    (a, b, 0, 1, 0)
                }
            }
            Instruction::Lt { src1, src2, .. } => {
                let (a, b) = get_ops(&prev_regs, *src1, *src2);
                (a, b, (a < b) as u64, 1, 0)
            }
            Instruction::Jz { cond, .. } => {
                let c = prev_regs[*cond as usize];
                (c, 0, 1, 1, 0)
            }
        };
        if !active {
            res = 0;
            quot = 1;
            wrap = 0;
        }
        cols[SRC1_COL][out_row] = Felt::from(s1);
        cols[SRC2_COL][out_row] = Felt::from(s2);
        cols[RES_COL][out_row] = Felt::from(res);
        cols[QUOT_COL][out_row] = Felt::from(quot);
        cols[COND_COL][out_row] = if matches!(instr, Instruction::Jz { .. }) {
            Felt::from(s1)
        } else {
            Felt::ZERO
        };

        // bit decomposition. lt/mod rows decompose a comparison diff, all others decompose res.
        let decomp_val = if !active {
            0
        } else if matches!(instr, Instruction::Lt { .. }) {
            if res == 1 {
                s2 - s1 - 1
            } else {
                s1 - s2
            }
        } else if matches!(instr, Instruction::Mod { .. }) {
            s2 - res - 1
        } else {
            res
        };
        for bit in 0..NUM_RANGE_BITS {
            cols[RANGE_BITS_BASE + bit][out_row] = Felt::from((decomp_val >> bit) & 1);
            cols[WRAP_BITS_BASE + bit][out_row] = Felt::from((wrap >> bit) & 1);
        }

        if active {
            skip_countdown = match instr {
                Instruction::Jz { offset, .. } if s1 == 0 => *offset,
                _ => 0,
            };
        } else {
            skip_countdown -= 1;
        }
    }

    if n > 0 {
        let last_regs = &vm_trace[n - 1].registers;
        for (r, col) in cols.iter_mut().enumerate().take(NUM_REGISTERS) {
            col[(n + 1)..trace_len].fill(Felt::from(last_regs[r]));
        }
        cols[QUOT_COL][(n + 1)..trace_len].fill(Felt::ONE);
    } else {
        cols[QUOT_COL][1..trace_len].fill(Felt::ONE);
    }
    for bit in 0..NUM_RANGE_BITS {
        for value in cols[RANGE_BITS_BASE + bit]
            .iter_mut()
            .take(trace_len)
            .skip(n + 1)
        {
            *value = Felt::ZERO;
        }
        for value in cols[WRAP_BITS_BASE + bit]
            .iter_mut()
            .take(trace_len)
            .skip(n + 1)
        {
            *value = Felt::ZERO;
        }
        if n + 1 < trace_len {
            cols[RANGE_BITS_BASE + bit][trace_len - 1] = Felt::ONE;
            cols[WRAP_BITS_BASE + bit][trace_len - 1] = Felt::ONE;
        }
    }
    cols[ACTIVE_COL][n..trace_len].fill(Felt::ONE);
    TraceTable::init(cols)
}
