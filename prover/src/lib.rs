pub mod public_inputs;
pub mod trace_builder;

use winterfell::math::fields::f128::BaseElement;

pub const NUM_REGISTERS: usize = 16;
pub const TRACE_WIDTH: usize = 19; // 16 regs + 3 witness cols
                                   // witness indices
pub const RES_COL: usize = 16;
pub const SRC1_COL: usize = 17;
pub const SRC2_COL: usize = 18;

pub type Felt = BaseElement;
