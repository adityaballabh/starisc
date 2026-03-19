pub(crate) mod air;
pub mod prover;
pub(crate) mod public_inputs;
pub(crate) mod trace_builder;

use winterfell::math::fields::f128::BaseElement;

pub(crate) const NUM_REGISTERS: usize = 16;
const NUM_WITNESS_COLS: usize = 3;
pub(crate) const NUM_RANGE_BITS: usize = 64;
// trace
// witness indices
pub(crate) const RES_COL: usize = 16;
pub(crate) const SRC1_COL: usize = 17;
pub(crate) const SRC2_COL: usize = 18;
// bit decomp [19, 82]. range check on each row. reused for lt diff on lt rows
pub(crate) const RANGE_BITS_BASE: usize = NUM_REGISTERS + NUM_WITNESS_COLS;
pub(crate) const TRACE_WIDTH: usize = NUM_REGISTERS + NUM_WITNESS_COLS + NUM_RANGE_BITS;

// constraints: regs [0,15], witness [16,18], assert_eq[19], range+lt [20,85]
pub(crate) const ASSERT_EQ_CON: usize = NUM_REGISTERS + NUM_WITNESS_COLS;
pub(crate) const RANGE_BITS_CON_BASE: usize = ASSERT_EQ_CON + 1; // bit boolean constraints
pub(crate) const LT_RES_BOOL_CON: usize = RANGE_BITS_CON_BASE + NUM_RANGE_BITS; // lt res must be 0 or 1
pub(crate) const RANGE_RECON_CON: usize = LT_RES_BOOL_CON + 1; // reconstruct lt diff or res (for non-lt)
pub(crate) const NUM_RANGE_LT_CONSTRAINTS: usize = NUM_RANGE_BITS + 2;
pub(crate) const NUM_CONSTRAINTS: usize =
    NUM_REGISTERS + NUM_WITNESS_COLS + 1 + NUM_RANGE_LT_CONSTRAINTS;

pub type Felt = BaseElement;
