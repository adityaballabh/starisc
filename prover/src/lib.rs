pub(crate) mod air;
pub mod prover;
pub(crate) mod public_inputs;
pub(crate) mod trace_builder;

use winterfell::math::fields::f128::BaseElement;

pub(crate) const NUM_REGISTERS: usize = 16;
const NUM_WITNESS_COLS: usize = 3;
pub(crate) const NUM_DIFF_BITS: usize = 64;
// trace
// witness indices
pub(crate) const RES_COL: usize = 16;
pub(crate) const SRC1_COL: usize = 17;
pub(crate) const SRC2_COL: usize = 18;
// bit decomp of lt diff [19, 82] (reused for mod)
pub(crate) const LT_BITS_BASE: usize = NUM_REGISTERS + NUM_WITNESS_COLS;
pub(crate) const TRACE_WIDTH: usize = NUM_REGISTERS + NUM_WITNESS_COLS + NUM_DIFF_BITS;

// constraints: regs [0,15], witness [16,18], assert_eq[19], lt [20,85]
pub(crate) const ASSERT_EQ_CON: usize = NUM_REGISTERS + NUM_WITNESS_COLS;
pub(crate) const LT_BITS_CON_BASE: usize = ASSERT_EQ_CON + 1; // boolean constraints for diff decomp
pub(crate) const LT_RES_BOOL_CON: usize = LT_BITS_CON_BASE + NUM_DIFF_BITS; // res must be in {0, 1}
pub(crate) const LT_DIFF_CON: usize = LT_RES_BOOL_CON + 1; // ensure bit columns represent the algebraic diff
pub(crate) const NUM_LT_CONSTRAINTS: usize = NUM_DIFF_BITS + 2;
pub(crate) const NUM_CONSTRAINTS: usize = NUM_REGISTERS + NUM_WITNESS_COLS + 1 + NUM_LT_CONSTRAINTS;

pub type Felt = BaseElement;
