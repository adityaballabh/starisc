pub(crate) mod air;
pub mod prover;
pub(crate) mod public_inputs;
pub(crate) mod trace_builder;

use winterfell::math::fields::f128::BaseElement;

pub(crate) const NUM_REGISTERS: usize = 16;
const NUM_WITNESS_COLS: usize = 3;
// witness indices
pub(crate) const RES_COL: usize = 16;
pub(crate) const SRC1_COL: usize = 17;
pub(crate) const SRC2_COL: usize = 18;
pub(crate) const TRACE_WIDTH: usize = NUM_REGISTERS + NUM_WITNESS_COLS;
// regs, witness cols, and assert_eq
pub(crate) const NUM_CONSTRAINTS: usize = NUM_REGISTERS + NUM_WITNESS_COLS + 1;

pub type Felt = BaseElement;
