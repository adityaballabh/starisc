use crate::{
    Felt, NUM_RANGE_BITS, NUM_REGISTERS, QUOT_COL, RANGE_BITS_BASE, RES_COL, SRC1_COL, SRC2_COL,
    TRACE_WIDTH,
};
use std::array::from_fn;
use vm::{Instruction, Trace};
use winterfell::math::{FieldElement, StarkField};
use winterfell::TraceTable;

pub fn get_trace_len(prog: &[Instruction]) -> usize {
    // +1 for initial row. winterfell restriction: min 8 and power of 2
    (prog.len() + 1).next_power_of_two().max(8)
}

fn get_ops(regs: &[u64; 16], s1: u8, s2: u8) -> (u64, u64) {
    (regs[s1 as usize], regs[s2 as usize])
}

fn perform_binary_op(regs: &[u64; 16], s1: u8, s2: u8, op: fn(u64, u64) -> u64) -> (u64, u64, u64) {
    let (a, b) = get_ops(regs, s1, s2);
    (a, b, op(a, b))
}

pub fn get_bits_used(prog: &[Instruction], vm_trace: &Trace) -> u64 {
    let mut bits: u64 = 0;
    let mut regs: [u64; 16] = [0; 16];
    for (instr, row) in prog.iter().zip(vm_trace.iter()) {
        let decomp_val = if let Instruction::Lt { src1, src2, .. } = instr {
            let (s1, s2) = get_ops(&regs, *src1, *src2);
            if s1 < s2 {
                s2 - s1 - 1
            } else {
                s1 - s2
            }
        } else if let Instruction::Mod { src1, src2, .. } = instr {
            let (_, s2) = get_ops(&regs, *src1, *src2);
            let res = row.registers[instr_dest(instr) as usize];
            debug_assert!(s2 != 0, "MOD by zero should not reach trace building");
            s2 - res - 1
        } else {
            // decomp res for range check (non-lt)
            match instr {
                Instruction::Set { dest, .. }
                | Instruction::Add { dest, .. }
                | Instruction::Sub { dest, .. }
                | Instruction::Mul { dest, .. }
                | Instruction::Mod { dest, .. } => row.registers[*dest as usize],
                Instruction::AssertEq { .. } => 1,
                Instruction::Lt { .. } => unreachable!(),
            }
        };
        bits |= decomp_val;
        regs = row.registers;
    }
    bits
}

fn instr_dest(instr: &Instruction) -> u8 {
    match instr {
        Instruction::Set { dest, .. }
        | Instruction::Add { dest, .. }
        | Instruction::Sub { dest, .. }
        | Instruction::Mul { dest, .. }
        | Instruction::Mod { dest, .. }
        | Instruction::Lt { dest, .. } => *dest,
        Instruction::AssertEq { .. } => unreachable!("ASSERT_EQ does not write a destination"),
    }
}

pub fn build_trace(prog: &[Instruction], vm_trace: &Trace) -> TraceTable<Felt> {
    assert_eq!(prog.len(), vm_trace.len());
    let n = prog.len();
    let trace_len = get_trace_len(prog);
    let mut cols = vec![vec![Felt::ZERO; trace_len]; TRACE_WIDTH];

    for (i, (instr, row)) in prog.iter().zip(vm_trace.iter()).enumerate() {
        let out_row = i + 1;

        for (r, col) in cols.iter_mut().enumerate().take(NUM_REGISTERS) {
            col[out_row] = Felt::from(row.registers[r]);
        }

        let prev_regs: [u64; 16] = from_fn(|r| cols[r][out_row - 1].as_int() as u64);

        let (s1, s2, res, quot) = match instr {
            Instruction::Set { val, .. } => (0, 0, *val, 0),
            Instruction::Add { src1, src2, .. } => {
                let (s1, s2, res) = perform_binary_op(&prev_regs, *src1, *src2, u64::wrapping_add);
                (s1, s2, res, 0)
            }
            Instruction::Sub { src1, src2, .. } => {
                let (s1, s2, res) = perform_binary_op(&prev_regs, *src1, *src2, u64::wrapping_sub);
                (s1, s2, res, 0)
            }
            Instruction::Mul { src1, src2, .. } => {
                let (s1, s2, res) = perform_binary_op(&prev_regs, *src1, *src2, u64::wrapping_mul);
                (s1, s2, res, 0)
            }
            Instruction::AssertEq { r1, r2 } => {
                let (a, b) = get_ops(&prev_regs, *r1, *r2);
                // Store 1 on ASSERT_EQ rows so the equality constraint keeps a stable degree.
                (a, b, 1, 0)
            }
            Instruction::Mod { src1, src2, .. } => {
                let (a, b) = get_ops(&prev_regs, *src1, *src2);
                debug_assert!(b != 0, "MOD by zero should not reach trace building");
                // Store quotient + 1 so the MOD quotient witness is never identically zero.
                (a, b, a % b, (a / b) + 1)
            }
            Instruction::Lt { src1, src2, .. } => {
                let (a, b) = get_ops(&prev_regs, *src1, *src2);
                (a, b, (a < b) as u64, 0)
            }
        };
        cols[SRC1_COL][out_row] = Felt::from(s1);
        cols[SRC2_COL][out_row] = Felt::from(s2);
        cols[RES_COL][out_row] = Felt::from(res);
        cols[QUOT_COL][out_row] = Felt::from(quot);

        // bit decomposition. lt/mod rows decompose a comparison diff, all others decompose res.
        let decomp_val = if matches!(instr, Instruction::Lt { .. }) {
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
        }
    }

    if n > 0 {
        let last_regs = &vm_trace[n - 1].registers;
        for (r, col) in cols.iter_mut().enumerate().take(NUM_REGISTERS) {
            col[(n + 1)..trace_len].fill(Felt::from(last_regs[r]));
        }
    }
    TraceTable::init(cols)
}
