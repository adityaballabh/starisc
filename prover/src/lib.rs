pub(crate) mod air;
pub mod prover;
pub(crate) mod public_inputs;
pub(crate) mod trace_builder;

use winterfell::math::fields::f128::BaseElement;
pub use winterfell::Proof;

pub(crate) const NUM_REGISTERS: usize = 16;
const NUM_WITNESS_COLS: usize = 8;
pub(crate) const NUM_RANGE_BITS: usize = 64;
// trace: regs [0,15], witnesses [16,23], range bits [24,87], wrap bits [88,151].
pub(crate) const RES_COL: usize = 16;
pub(crate) const SRC1_COL: usize = 17;
pub(crate) const SRC2_COL: usize = 18;
pub(crate) const QUOT_COL: usize = 19;
pub(crate) const ACTIVE_COL: usize = 20;
pub(crate) const COND_COL: usize = 21;
pub(crate) const SKIP_COUNTDOWN_COL: usize = 22;
pub(crate) const SKIP_COUNTDOWN_INV_COL: usize = 23;
// bit decomp for res/lt/mod diff [24,87].
pub(crate) const RANGE_BITS_BASE: usize = NUM_REGISTERS + NUM_WITNESS_COLS;
// bit decomp for wrapping carries/borrows/high limbs [88,151].
pub(crate) const WRAP_BITS_BASE: usize = RANGE_BITS_BASE + NUM_RANGE_BITS;
pub(crate) const TRACE_WIDTH: usize = NUM_REGISTERS + NUM_WITNESS_COLS + (2 * NUM_RANGE_BITS);

// constraints: regs [0,15], witness [16,23], assert_eq/mod/branch [24,31],
// range bits [32,95], wrap bits [96,159], range/lt [160,161].
pub(crate) const ASSERT_EQ_CON: usize = NUM_REGISTERS + NUM_WITNESS_COLS;
pub(crate) const MOD_REL_CON: usize = ASSERT_EQ_CON + 1;
pub(crate) const ACTIVE_BOOL_CON: usize = MOD_REL_CON + 1;
pub(crate) const COND_BOOL_CON: usize = ACTIVE_BOOL_CON + 1;
pub(crate) const COND_MATCH_CON: usize = COND_BOOL_CON + 1;
pub(crate) const SKIP_ACTIVE_ZERO_CON: usize = COND_MATCH_CON + 1;
pub(crate) const SKIP_COUNTDOWN_INV_CON: usize = SKIP_ACTIVE_ZERO_CON + 1;
pub(crate) const SKIP_COUNTDOWN_CON: usize = SKIP_COUNTDOWN_INV_CON + 1;
pub(crate) const RANGE_BITS_CON_BASE: usize = SKIP_COUNTDOWN_CON + 1; // bit boolean constraints
pub(crate) const WRAP_BITS_CON_BASE: usize = RANGE_BITS_CON_BASE + NUM_RANGE_BITS;
pub(crate) const LT_RES_BOOL_CON: usize = WRAP_BITS_CON_BASE + NUM_RANGE_BITS; // lt res must be 0 or 1
pub(crate) const RANGE_RECON_CON: usize = LT_RES_BOOL_CON + 1; // reconstruct lt diff or res (for non-lt)
pub(crate) const NUM_RANGE_LT_CONSTRAINTS: usize = (2 * NUM_RANGE_BITS) + 2;
pub(crate) const NUM_CONSTRAINTS: usize =
    NUM_REGISTERS + NUM_WITNESS_COLS + 8 + NUM_RANGE_LT_CONSTRAINTS;

pub type Felt = BaseElement;
