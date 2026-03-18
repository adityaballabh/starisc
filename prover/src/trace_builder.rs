use crate::{
    Felt, LT_BITS_BASE, NUM_DIFF_BITS, NUM_REGISTERS, RES_COL, SRC1_COL, SRC2_COL, TRACE_WIDTH,
};
use std::array::from_fn;
use vm::{Instruction, Trace};
use winterfell::math::{FieldElement, StarkField};
use winterfell::TraceTable;

// +1 for initial row. winterfell restriction: min 8 and power of 2
pub fn get_trace_len(prog: &[Instruction]) -> usize {
    (prog.len() + 1).next_power_of_two().max(8)
}

pub fn get_ops(regs: &[u64; 16], s1: u8, s2: u8) -> (u64, u64) {
    (regs[s1 as usize], regs[s2 as usize])
}

fn perform_binary_op(regs: &[u64; 16], s1: u8, s2: u8, op: fn(u64, u64) -> u64) -> (u64, u64, u64) {
    let (a, b) = get_ops(regs, s1, s2);
    (a, b, op(a, b))
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

        let (s1, s2, res) = match instr {
            Instruction::Set { val, .. } => (0, 0, *val),
            Instruction::Add { src1, src2, .. } => {
                perform_binary_op(&prev_regs, *src1, *src2, u64::wrapping_add)
            }
            Instruction::Sub { src1, src2, .. } => {
                perform_binary_op(&prev_regs, *src1, *src2, u64::wrapping_sub)
            }
            Instruction::Mul { src1, src2, .. } => {
                perform_binary_op(&prev_regs, *src1, *src2, u64::wrapping_mul)
            }
            Instruction::AssertEq { r1, r2 } => {
                let (a, b) = get_ops(&prev_regs, *r1, *r2);
                (a, b, 0)
            }
            Instruction::Mod { src1, src2, .. } => {
                perform_binary_op(&prev_regs, *src1, *src2, u64::wrapping_rem)
            }
            Instruction::Lt { src1, src2, .. } => {
                let (a, b) = get_ops(&prev_regs, *src1, *src2);
                (a, b, (a < b) as u64)
            }
        };
        cols[SRC1_COL][out_row] = Felt::from(s1);
        cols[SRC2_COL][out_row] = Felt::from(s2);
        cols[RES_COL][out_row] = Felt::from(res);

        // bit decomposition of diff for lt rows
        if matches!(instr, Instruction::Lt { .. }) {
            let diff = if res == 1 { s2 - s1 - 1 } else { s1 - s2 };
            for bit in 0..NUM_DIFF_BITS {
                cols[LT_BITS_BASE + bit][out_row] = Felt::from((diff >> bit) & 1);
            }
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
