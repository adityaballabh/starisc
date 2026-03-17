pub mod air;
pub mod public_inputs;
pub mod trace_builder;

use winterfell::math::fields::f128::BaseElement;

pub const NUM_REGISTERS: usize = 16;
const NUM_WITNESS_COLS: usize = 3;
// witness indices
pub const RES_COL: usize = 16;
pub const SRC1_COL: usize = 17;
pub const SRC2_COL: usize = 18;
pub const TRACE_WIDTH: usize = NUM_REGISTERS + NUM_WITNESS_COLS;
// regs, witness cols, and assert_eq
pub const NUM_CONSTRAINTS: usize = NUM_REGISTERS + NUM_WITNESS_COLS + 1; 

pub type Felt = BaseElement;
