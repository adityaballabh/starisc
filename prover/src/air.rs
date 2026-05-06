use crate::public_inputs::{
    PublicInputs, P_CONST, P_IS_ADD, P_IS_ASSERT_EQ, P_IS_JZ, P_IS_LT, P_IS_MOD, P_IS_MUL,
    P_IS_NOP, P_IS_READ_PUB, P_IS_SET, P_IS_SUB, P_OFFSET, P_RES_BASE, P_SRC1_BASE, P_SRC2_BASE,
};
use crate::{
    Felt, ACTIVE_BOOL_CON, ACTIVE_COL, ASSERT_EQ_CON, COND_BOOL_CON, COND_MATCH_CON,
    LT_RES_BOOL_CON, MOD_REL_CON, NUM_CONSTRAINTS, NUM_RANGE_BITS, NUM_REGISTERS, QUOT_COL,
    RANGE_BITS_BASE, RANGE_BITS_CON_BASE, RANGE_RECON_CON, RES_COL, SKIP_ACTIVE_ZERO_CON,
    SKIP_COUNTDOWN_COL, SKIP_COUNTDOWN_CON, SKIP_COUNTDOWN_INV_COL, SKIP_COUNTDOWN_INV_CON,
    SRC1_COL, SRC2_COL, TRACE_WIDTH, WRAP_BITS_BASE, WRAP_BITS_CON_BASE,
};
use winterfell::math::FieldElement;
use winterfell::{
    Air, AirContext, Assertion, EvaluationFrame, ProofOptions, TraceInfo,
    TransitionConstraintDegree,
};

pub struct VmAir {
    context: AirContext<Felt>,
    public_inputs: PublicInputs,
}

impl Air for VmAir {
    type BaseField = Felt;
    type PublicInputs = PublicInputs;
    type GkrProof = ();
    type GkrVerifier = ();

    fn new(trace_info: TraceInfo, pub_inputs: PublicInputs, options: ProofOptions) -> Self {
        let trace_len = pub_inputs.trace_len;
        let has_result_constraint = pub_inputs.has_result_constraint();

        // new(1) is the default constraint. use cyclic for periodic/instruction-specific constraints
        // new(2) if the entire column has a degree 2 constraint
        let cyclic = |base| TransitionConstraintDegree::with_cycles(base, vec![trace_len]);
        let mut degrees = vec![TransitionConstraintDegree::new(1); NUM_CONSTRAINTS];
        for (j, degree) in degrees.iter_mut().enumerate().take(NUM_REGISTERS) {
            if pub_inputs.dest_mask[j] {
                *degree = cyclic(if pub_inputs.has_taken_jz { 2 } else { 1 });
            }
        }
        if pub_inputs.has_nonzero_src1 {
            degrees[SRC1_COL] = cyclic(1);
        }
        if pub_inputs.has_nonzero_src2 {
            degrees[SRC2_COL] = cyclic(1);
        }
        degrees[RES_COL] = if pub_inputs.has_taken_jz && pub_inputs.has_mul {
            cyclic(3)
        } else if (pub_inputs.has_taken_jz && has_result_constraint) || pub_inputs.has_mul {
            cyclic(2)
        } else if has_result_constraint {
            cyclic(1)
        } else {
            TransitionConstraintDegree::new(1)
        };
        if pub_inputs.has_mod {
            degrees[QUOT_COL] = cyclic(1);
            degrees[MOD_REL_CON] = cyclic(if pub_inputs.has_taken_jz { 3 } else { 2 });
        }
        if pub_inputs.has_assert_eq {
            degrees[ASSERT_EQ_CON] = cyclic(if pub_inputs.has_taken_jz { 2 } else { 1 });
        }
        // branch degrees depend on whether taken/not-taken rows appear in this trace
        let has_taken_branch = pub_inputs.has_taken_jz;
        let has_fallthrough_branch = pub_inputs.has_not_taken_jz;
        degrees[ACTIVE_BOOL_CON] = if has_taken_branch {
            TransitionConstraintDegree::new(2)
        } else {
            TransitionConstraintDegree::new(1)
        };
        if pub_inputs.has_jz {
            degrees[COND_BOOL_CON] = if has_fallthrough_branch {
                cyclic(if has_taken_branch { 3 } else { 2 })
            } else {
                TransitionConstraintDegree::new(1)
            };
            if has_taken_branch || has_fallthrough_branch {
                degrees[SKIP_COUNTDOWN_CON] = cyclic(if has_taken_branch { 3 } else { 2 });
                if has_taken_branch {
                    degrees[SKIP_ACTIVE_ZERO_CON] = TransitionConstraintDegree::new(2);
                    degrees[SKIP_COUNTDOWN_INV_CON] = cyclic(2);
                }
            }
        }
        for i in 0..NUM_RANGE_BITS {
            if pub_inputs.bits_used & (1u64 << i) != 0 {
                degrees[RANGE_BITS_CON_BASE + i] = TransitionConstraintDegree::new(2);
            }
            if pub_inputs.wrap_bits_used & (1u64 << i) != 0 {
                degrees[WRAP_BITS_CON_BASE + i] = TransitionConstraintDegree::new(2);
            }
        }
        if pub_inputs.has_lt {
            degrees[LT_RES_BOOL_CON] = cyclic(if pub_inputs.has_taken_jz { 3 } else { 2 });
            degrees[RANGE_RECON_CON] = cyclic(if pub_inputs.has_taken_jz { 3 } else { 2 });
        } else if pub_inputs.has_mod || has_result_constraint {
            degrees[RANGE_RECON_CON] = cyclic(if pub_inputs.has_taken_jz { 2 } else { 1 });
        }

        let num_assertions = TRACE_WIDTH;
        let context = AirContext::new(trace_info, degrees, num_assertions, options);
        Self {
            context,
            public_inputs: pub_inputs,
        }
    }

    fn context(&self) -> &AirContext<Felt> {
        &self.context
    }

    // result stores constraint residuals (each must be 0)
    fn evaluate_transition<E: FieldElement<BaseField = Felt>>(
        &self,
        frame: &EvaluationFrame<E>,
        periodic_values: &[E],
        result: &mut [E],
    ) {
        let curr_row = frame.current();
        let next_row = frame.next();
        let curr_pub_in = periodic_values;

        let next_src1 = next_row[SRC1_COL];
        let next_src2 = next_row[SRC2_COL];
        let next_res = next_row[RES_COL];
        let next_quot = next_row[QUOT_COL];
        let active = curr_row[ACTIVE_COL];
        let cond = next_src1;
        let skip_countdown = curr_row[SKIP_COUNTDOWN_COL];
        let skip_countdown_inv = curr_row[SKIP_COUNTDOWN_INV_COL];
        let two64 = E::from(Felt::from(u64::MAX)) + E::ONE;
        let mut wrap_bit_sum = E::ZERO;
        for i in 0..NUM_RANGE_BITS {
            wrap_bit_sum += E::from(Felt::from(1u64 << i)) * next_row[WRAP_BITS_BASE + i];
        }
        // This vanishes on the base trace domain because `P_IS_NOP` is boolean there.
        // On the LDE domain it keeps Winterfell's exact degree checks tied to public
        // program shape instead of private branch/source values.
        let degree_anchor = curr_pub_in[P_IS_NOP] * (curr_pub_in[P_IS_NOP] - E::ONE);
        let degree_31 = degree_anchor;
        let degree_62 = degree_31 * next_row[RANGE_BITS_BASE];
        let degree_93 = degree_62 * next_row[WRAP_BITS_BASE];

        // all reg except dest should not change. dest should be next_res
        for j in 0..NUM_REGISTERS {
            result[j] = (next_row[j] - curr_row[j])
                - active * curr_pub_in[P_RES_BASE + j] * (next_res - curr_row[j]);
            if self.public_inputs.has_taken_jz && self.public_inputs.dest_mask[j] {
                result[j] += degree_62;
            }
        }

        let is_add = curr_pub_in[P_IS_ADD];
        let is_sub = curr_pub_in[P_IS_SUB];
        let is_mul = curr_pub_in[P_IS_MUL];
        let is_lt = curr_pub_in[P_IS_LT];
        let is_mod = curr_pub_in[P_IS_MOD];
        let is_jz = curr_pub_in[P_IS_JZ];
        let is_assert_eq = curr_pub_in[P_IS_ASSERT_EQ];
        let is_range_checked_res =
            curr_pub_in[P_IS_SET] + is_add + is_sub + is_mul + is_assert_eq + is_jz;

        result[RES_COL] = active
            * (curr_pub_in[P_IS_SET] * (next_res - curr_pub_in[P_CONST])
                + curr_pub_in[P_IS_READ_PUB] * (next_res - curr_pub_in[P_CONST])
                + is_add * (next_res + wrap_bit_sum * two64 - next_src1 - next_src2)
                + is_sub * (next_res - next_src1 + next_src2 - wrap_bit_sum * two64)
                + is_mul * (next_res + wrap_bit_sum * two64 - next_src1 * next_src2)
                + is_assert_eq * (next_res - E::ONE)
                + is_jz * (next_res - E::ONE));
        if self.public_inputs.has_taken_jz {
            if self.public_inputs.has_mul {
                result[RES_COL] += degree_93;
            } else if self.public_inputs.has_result_constraint() {
                result[RES_COL] += degree_62;
            }
        }

        // next[src1/2] should be the dot product of their reg selectors and curr regs
        let (mut exp_s1, mut exp_s2) = (E::ZERO, E::ZERO);
        for j in 0..NUM_REGISTERS {
            exp_s1 += curr_pub_in[P_SRC1_BASE + j] * curr_row[j];
            exp_s2 += curr_pub_in[P_SRC2_BASE + j] * curr_row[j];
        }
        result[SRC1_COL] = next_src1 - exp_s1;
        result[SRC2_COL] = next_src2 - exp_s2;
        if self.public_inputs.has_nonzero_src1 {
            result[SRC1_COL] += degree_31;
        }
        if self.public_inputs.has_nonzero_src2 {
            result[SRC2_COL] += degree_31;
        }
        result[ASSERT_EQ_CON] =
            active * is_assert_eq * (next_src1 - next_src2 - (next_res - E::ONE));
        if self.public_inputs.has_taken_jz && self.public_inputs.has_assert_eq {
            result[ASSERT_EQ_CON] += degree_62;
        }
        if self.public_inputs.has_mod {
            result[QUOT_COL] = (E::ONE - is_mod) * (next_quot - E::ONE);
            result[MOD_REL_CON] =
                active * is_mod * (next_src1 - (next_src2 * (next_quot - E::ONE) + next_res));
            if self.public_inputs.has_taken_jz {
                result[MOD_REL_CON] += degree_93;
            }
        } else {
            result[QUOT_COL] = next_quot - E::ONE;
            result[MOD_REL_CON] = E::ZERO;
        }

        // 64 bit boolean constraints. enforced on each row for range checking
        for i in 0..NUM_RANGE_BITS {
            let bit = next_row[RANGE_BITS_BASE + i];
            result[RANGE_BITS_CON_BASE + i] = bit * (bit - E::ONE);
            let wrap_bit = next_row[WRAP_BITS_BASE + i];
            result[WRAP_BITS_CON_BASE + i] = wrap_bit * (wrap_bit - E::ONE);
        }

        // lt res should be 0 or 1
        result[LT_RES_BOOL_CON] = active * is_lt * next_res * (next_res - E::ONE);

        let has_taken_branch = self.public_inputs.has_taken_jz;
        let has_fallthrough_branch = self.public_inputs.has_not_taken_jz;

        result[ACTIVE_BOOL_CON] = if has_taken_branch {
            active * (active - E::ONE)
        } else {
            active - E::ONE
        };
        if has_taken_branch {
            result[ACTIVE_BOOL_CON] += degree_31;
        }
        result[COND_BOOL_CON] = if has_fallthrough_branch {
            active * is_jz * cond * (cond - E::ONE)
        } else {
            active * is_jz * cond
        };
        if has_taken_branch && has_fallthrough_branch {
            result[COND_BOOL_CON] += degree_93;
        } else if has_fallthrough_branch {
            result[COND_BOOL_CON] += degree_62;
        }
        result[COND_MATCH_CON] = E::ZERO;
        if has_taken_branch {
            result[SKIP_ACTIVE_ZERO_CON] = active * skip_countdown;
            result[SKIP_COUNTDOWN_INV_CON] =
                (E::ONE - active) * (skip_countdown * skip_countdown_inv - E::ONE);
            result[SKIP_ACTIVE_ZERO_CON] += degree_31;
            result[SKIP_COUNTDOWN_INV_CON] += degree_62;
        } else {
            result[SKIP_ACTIVE_ZERO_CON] = E::ZERO;
            result[SKIP_COUNTDOWN_INV_CON] = E::ZERO;
        }
        let taken = active * is_jz * (E::ONE - cond);
        result[SKIP_COUNTDOWN_CON] = next_row[SKIP_COUNTDOWN_COL]
            - (taken * curr_pub_in[P_OFFSET] + (E::ONE - active) * (skip_countdown - E::ONE));
        if has_taken_branch {
            result[SKIP_COUNTDOWN_CON] += degree_93;
        } else if has_fallthrough_branch {
            result[SKIP_COUNTDOWN_CON] += degree_62;
        }

        // combined reconstruction. lt rows decompose comparison diff, mod rows decompose src2-res-1,
        // and all other rows decompose res.
        let exp_diff = next_res * (next_src2 - next_src1 - E::ONE)
            + (E::ONE - next_res) * (next_src1 - next_src2);
        let mod_diff = next_src2 - next_res - E::ONE;
        let mut bit_sum = E::ZERO;
        for i in 0..NUM_RANGE_BITS {
            bit_sum += E::from(Felt::from(1u64 << i)) * next_row[RANGE_BITS_BASE + i];
        }
        result[RANGE_RECON_CON] = active
            * (is_lt * (exp_diff - bit_sum)
                + is_mod * (mod_diff - bit_sum)
                + is_range_checked_res * (next_res - bit_sum));
        if self.public_inputs.has_taken_jz {
            if self.public_inputs.has_lt {
                result[LT_RES_BOOL_CON] += degree_93;
                result[RANGE_RECON_CON] += degree_93;
            } else if self.public_inputs.has_mod || self.public_inputs.has_result_constraint() {
                result[RANGE_RECON_CON] += degree_62;
            }
        }
    }

    // all trace cols should be 0 for row 0
    fn get_assertions(&self) -> Vec<Assertion<Felt>> {
        let mut assertions: Vec<_> = (0..TRACE_WIDTH)
            .filter(|&col| col != ACTIVE_COL)
            .map(|col| Assertion::single(col, 0, Felt::ZERO))
            .collect();
        assertions.push(Assertion::single(ACTIVE_COL, 0, Felt::ONE));
        assertions
    }

    fn get_periodic_column_values(&self) -> Vec<Vec<Felt>> {
        self.public_inputs.build_periodic_columns()
    }
}
