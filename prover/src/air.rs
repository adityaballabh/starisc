use crate::public_inputs::{
    PublicInputs, P_CONST, P_IS_ADD, P_IS_ASSERT_EQ, P_IS_LT, P_IS_MUL, P_IS_SET, P_IS_SUB,
    P_RES_BASE, P_SRC1_BASE, P_SRC2_BASE,
};
use crate::{
    Felt, ASSERT_EQ_CON, LT_BITS_BASE, LT_BITS_CON_BASE, LT_DIFF_CON, LT_RES_BOOL_CON,
    NUM_CONSTRAINTS, NUM_DIFF_BITS, NUM_REGISTERS, RES_COL, SRC1_COL, SRC2_COL, TRACE_WIDTH,
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

        // constraints use periodic columns by default
        // use new(1) only if the periodic column is all zeros (which makes the constraint polynomial 0)
        let cyclic = |base| TransitionConstraintDegree::with_cycles(base, vec![trace_len]);
        let mut degrees = vec![cyclic(1); NUM_CONSTRAINTS];
        for (j, degree) in degrees.iter_mut().enumerate().take(NUM_REGISTERS) {
            if !pub_inputs.dest_mask[j] {
                *degree = TransitionConstraintDegree::new(1);
            }
        }
        if pub_inputs.has_mul {
            degrees[RES_COL] = cyclic(2);
        }
        if !pub_inputs.has_assert_eq {
            degrees[ASSERT_EQ_CON] = TransitionConstraintDegree::new(1);
        }
        // cyclic constraint only if the corresponding diff bit is non-zero at least once in the program
        for i in 0..NUM_DIFF_BITS {
            degrees[LT_BITS_CON_BASE + i] = if pub_inputs.diff_bits_used & (1u64 << i) != 0 {
                cyclic(2)
            } else {
                TransitionConstraintDegree::new(1)
            };
        }
        for idx in [LT_RES_BOOL_CON, LT_DIFF_CON] {
            degrees[idx] = if pub_inputs.has_lt {
                cyclic(2)
            } else {
                TransitionConstraintDegree::new(1)
            };
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

        // all reg except dest should not change. dest should be next_res
        for j in 0..NUM_REGISTERS {
            result[j] = (next_row[j] - curr_row[j])
                - curr_pub_in[P_RES_BASE + j] * (next_res - curr_row[j]);
        }

        // next[res] should be (is_set*const_val + is_add*(s1+s2) + is_sub*(s1-s2) + is_mul*s1*s2)
        let exp_res = curr_pub_in[P_IS_SET] * curr_pub_in[P_CONST] + curr_pub_in[P_IS_ADD] * (next_src1 + next_src2)
                    + curr_pub_in[P_IS_SUB] * (next_src1 - next_src2) + curr_pub_in[P_IS_MUL] * next_src1 * next_src2
                    // res_col will not enforce lt constraints (handled separately)
                    + curr_pub_in[P_IS_LT] * next_res;
        result[RES_COL] = next_res - exp_res;

        // next[src1/2] should be the dot product of their reg selectors and curr regs
        let (mut exp_s1, mut exp_s2) = (E::ZERO, E::ZERO);
        for j in 0..NUM_REGISTERS {
            exp_s1 += curr_pub_in[P_SRC1_BASE + j] * curr_row[j];
            exp_s2 += curr_pub_in[P_SRC2_BASE + j] * curr_row[j];
        }
        result[SRC1_COL] = next_src1 - exp_s1;
        result[SRC2_COL] = next_src2 - exp_s2;

        result[ASSERT_EQ_CON] = curr_pub_in[P_IS_ASSERT_EQ] * (next_src1 - next_src2);

        let is_lt = curr_pub_in[P_IS_LT];
        // 64 bit boolean constraints
        for i in 0..NUM_DIFF_BITS {
            let bit = next_row[LT_BITS_BASE + i];
            result[LT_BITS_CON_BASE + i] = is_lt * bit * (bit - E::ONE);
        }

        // res should be 0 or 1
        result[LT_RES_BOOL_CON] = is_lt * next_res * (next_res - E::ONE);

        // diff reconstruction. bit decomposition should match algebraic diff
        let exp_diff = next_res * (next_src2 - next_src1 - E::ONE)
            + (E::ONE - next_res) * (next_src1 - next_src2);
        let mut bit_sum = E::ZERO;
        for i in 0..NUM_DIFF_BITS {
            bit_sum += E::from(Felt::from(1u64 << i)) * next_row[LT_BITS_BASE + i];
        }
        result[LT_DIFF_CON] = is_lt * (exp_diff - bit_sum);
    }

    // all trace cols should be 0 for row 0
    fn get_assertions(&self) -> Vec<Assertion<Felt>> {
        (0..TRACE_WIDTH)
            .map(|col| Assertion::single(col, 0, Felt::ZERO))
            .collect()
    }

    fn get_periodic_column_values(&self) -> Vec<Vec<Felt>> {
        self.public_inputs.build_periodic_columns()
    }
}
