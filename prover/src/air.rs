use crate::public_inputs::{
    PublicInputs, P_CONST, P_IS_ADD, P_IS_ASSERT_EQ, P_IS_MUL, P_IS_SET, P_IS_SUB, P_RES_BASE,
    P_SRC1_BASE, P_SRC2_BASE,
};
use crate::{Felt, NUM_CONSTRAINTS, NUM_REGISTERS, RES_COL, SRC1_COL, SRC2_COL, TRACE_WIDTH};
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

        // all constraints are degree 1, except res (RES_COL) which can have degree 2 (for MUL)
        let mut degrees =
            vec![TransitionConstraintDegree::with_cycles(1, vec![trace_len]); NUM_CONSTRAINTS];
        degrees[RES_COL] = TransitionConstraintDegree::with_cycles(2, vec![trace_len]);

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
        let exp_res = curr_pub_in[P_IS_SET] * curr_pub_in[P_CONST]
            + curr_pub_in[P_IS_ADD] * (next_src1 + next_src2)
            + curr_pub_in[P_IS_SUB] * (next_src1 - next_src2)
            + curr_pub_in[P_IS_MUL] * next_src1 * next_src2;
        result[RES_COL] = next_res - exp_res;

        // next[src1/2] should be the dot product of their reg selectors and curr regs
        let (mut exp_s1, mut exp_s2) = (E::ZERO, E::ZERO);
        for j in 0..NUM_REGISTERS {
            exp_s1 += curr_pub_in[P_SRC1_BASE + j] * curr_row[j];
            exp_s2 += curr_pub_in[P_SRC2_BASE + j] * curr_row[j];
        }
        result[SRC1_COL] = next_src1 - exp_s1;
        result[SRC2_COL] = next_src2 - exp_s2;

        // src1 and src2 should be equal for is_assert_eq
        result[NUM_CONSTRAINTS - 1] = curr_pub_in[P_IS_ASSERT_EQ] * (next_src1 - next_src2);
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
