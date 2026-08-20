use crate::{
    Felt, ACTIVE_COL, COND_COL, MOD_RES_BITS_BASE, NUM_RANGE_BITS, NUM_REGISTERS, QUOT_COL,
    RANGE_BITS_BASE, RES_COL, SKIP_COUNTDOWN_COL, SKIP_COUNTDOWN_INV_COL, SRC1_COL, SRC2_COL,
    TRACE_WIDTH, WRAP_BITS_BASE,
};
use std::array::from_fn;
use vm::{Instruction, Trace};
use winterfell::math::{FieldElement, StarkField};
use winterfell::TraceTable;

const NO_SRC: u64 = 0;
const NO_QUOT: u64 = 0;
const NO_WRAP: u64 = 0;
const ASSERT_OK: u64 = 1;
const JZ_RES: u64 = 1;
const INACTIVE: u64 = 0;
const INACTIVE_RANGE: u64 = 0;

pub fn get_trace_len(prog: &[Instruction]) -> usize {
    // +1 for initial row. winterfell restriction: min 8 and power of 2
    (prog.len() + 1).next_power_of_two().max(32)
}

fn get_ops(regs: &[u64; 16], left_reg: u8, right_reg: u8) -> (u64, u64) {
    (regs[left_reg as usize], regs[right_reg as usize])
}

fn perform_binary_op(
    regs: &[u64; 16],
    left_reg: u8,
    right_reg: u8,
    op: fn(u64, u64) -> u64,
) -> (u64, u64, u64) {
    let (left, right) = get_ops(regs, left_reg, right_reg);
    (left, right, op(left, right))
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

        let (src1_val, src2_val, mut res, mut quot, mut wrap) = match instr {
            Instruction::Set { val, .. } => (NO_SRC, NO_SRC, *val, NO_QUOT, NO_WRAP),
            Instruction::ReadPriv { dest, .. } | Instruction::ReadPub { dest, .. } => (
                NO_SRC,
                NO_SRC,
                row.registers[*dest as usize],
                NO_QUOT,
                NO_WRAP,
            ),
            Instruction::Add { src1, src2, .. } => {
                let (left, right, res) =
                    perform_binary_op(&prev_regs, *src1, *src2, u64::wrapping_add);
                (left, right, res, NO_QUOT, add_wrap(left, right))
            }
            Instruction::Sub { src1, src2, .. } => {
                let (left, right, res) =
                    perform_binary_op(&prev_regs, *src1, *src2, u64::wrapping_sub);
                (left, right, res, NO_QUOT, sub_wrap(left, right))
            }
            Instruction::Mul { src1, src2, .. } => {
                let (left, right, res) =
                    perform_binary_op(&prev_regs, *src1, *src2, u64::wrapping_mul);
                (left, right, res, NO_QUOT, mul_wrap(left, right))
            }
            Instruction::AssertEq { r1, r2 } => {
                let (left, right) = get_ops(&prev_regs, *r1, *r2);
                // Store 1 on ASSERT_EQ rows so the equality constraint keeps a stable degree.
                (left, right, ASSERT_OK, NO_QUOT, NO_WRAP)
            }
            Instruction::Mod { src1, src2, .. } => {
                let (left, right) = get_ops(&prev_regs, *src1, *src2);
                if active {
                    let quotient = left / right;
                    (left, right, left % right, quotient, quotient)
                } else {
                    (left, right, INACTIVE, NO_QUOT, NO_WRAP)
                }
            }
            Instruction::Lt { src1, src2, .. } => {
                let (left, right) = get_ops(&prev_regs, *src1, *src2);
                (left, right, (left < right) as u64, NO_QUOT, NO_WRAP)
            }
            Instruction::Jz { cond, .. } => {
                let cond_val = prev_regs[*cond as usize];
                (cond_val, NO_SRC, JZ_RES, NO_QUOT, NO_WRAP)
            }
        };
        if !active {
            res = INACTIVE;
            quot = NO_QUOT;
            wrap = NO_WRAP;
        }
        cols[SRC1_COL][out_row] = Felt::from(src1_val);
        cols[SRC2_COL][out_row] = Felt::from(src2_val);
        cols[RES_COL][out_row] = Felt::from(res);
        // Offset q so Winterfell's exact degree check stays stable when every quotient is zero.
        cols[QUOT_COL][out_row] = Felt::from(quot) + Felt::ONE;
        cols[COND_COL][out_row] = if matches!(instr, Instruction::Jz { .. }) {
            Felt::from(src1_val)
        } else {
            Felt::ZERO
        };

        // bit decomposition. lt/mod rows decompose a comparison diff, all others decompose res.
        let decomp_val = if !active {
            INACTIVE_RANGE
        } else if matches!(instr, Instruction::Lt { .. }) {
            if res == 1 {
                src2_val - src1_val - 1
            } else {
                src1_val - src2_val
            }
        } else if matches!(instr, Instruction::Mod { .. }) {
            src2_val - res - 1
        } else {
            res
        };
        let mod_res = if active && matches!(instr, Instruction::Mod { .. }) {
            res
        } else {
            0
        };
        for bit in 0..NUM_RANGE_BITS {
            cols[RANGE_BITS_BASE + bit][out_row] = Felt::from((decomp_val >> bit) & 1);
            cols[WRAP_BITS_BASE + bit][out_row] = Felt::from((wrap >> bit) & 1);
            cols[MOD_RES_BITS_BASE + bit][out_row] = Felt::from((mod_res >> bit) & 1);
        }

        if active {
            skip_countdown = match instr {
                Instruction::Jz { offset, .. } if src1_val == 0 => *offset,
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
        for value in cols[MOD_RES_BITS_BASE + bit]
            .iter_mut()
            .take(trace_len)
            .skip(n + 1)
        {
            *value = Felt::ZERO;
        }
        if n + 1 < trace_len {
            cols[RANGE_BITS_BASE + bit][trace_len - 1] = Felt::ONE;
            cols[WRAP_BITS_BASE + bit][trace_len - 1] = Felt::ONE;
            cols[MOD_RES_BITS_BASE + bit][trace_len - 1] = Felt::ONE;
        }
    }
    cols[ACTIVE_COL][n..trace_len].fill(Felt::ONE);
    TraceTable::init(cols)
}
