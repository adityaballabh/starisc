use crate::public_inputs::{
    PublicInputs, P_CONST, P_IS_ADD, P_IS_ASSERT_EQ, P_IS_LT, P_IS_MOD, P_IS_MUL, P_IS_SET,
    P_IS_SUB, P_RES_BASE, P_SRC1_BASE, P_SRC2_BASE,
};
use crate::{
    Felt, ASSERT_EQ_CON, LT_RES_BOOL_CON, MOD_REL_CON, NUM_CONSTRAINTS, NUM_RANGE_BITS,
    NUM_REGISTERS, QUOT_COL, RANGE_BITS_BASE, RANGE_BITS_CON_BASE, RANGE_RECON_CON, RES_COL,
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
        let has_result_constraint = pub_inputs.prog.iter().any(|instr| {
            matches!(
                instr,
                vm::Instruction::Set { .. }
                    | vm::Instruction::Add { .. }
                    | vm::Instruction::Sub { .. }
                    | vm::Instruction::Mul { .. }
                    | vm::Instruction::AssertEq { .. }
            )
        });

        // new(1) is the default constraint. use cyclic for periodic/instruction-specific constraints
        // new(2) if the entire column has a degree 2 constraint
        let cyclic = |base| TransitionConstraintDegree::with_cycles(base, vec![trace_len]);
        let mut degrees = vec![TransitionConstraintDegree::new(1); NUM_CONSTRAINTS];
        for (j, degree) in degrees.iter_mut().enumerate().take(NUM_REGISTERS) {
            if pub_inputs.dest_mask[j] {
                *degree = cyclic(1);
            }
        }
        if pub_inputs.has_nonzero_src1 {
            degrees[SRC1_COL] = cyclic(1);
        }
        if pub_inputs.has_nonzero_src2 {
            degrees[SRC2_COL] = cyclic(1);
        }
        degrees[RES_COL] = if pub_inputs.has_mul {
            cyclic(2)
        } else if has_result_constraint {
            cyclic(1)
        } else {
            TransitionConstraintDegree::new(1)
        };
        if pub_inputs.has_mod {
            degrees[QUOT_COL] = cyclic(1);
            degrees[MOD_REL_CON] = cyclic(2);
        }
        if pub_inputs.has_assert_eq {
            degrees[ASSERT_EQ_CON] = cyclic(1);
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
            degrees[LT_RES_BOOL_CON] = cyclic(2);
            degrees[RANGE_RECON_CON] = cyclic(2);
        } else if pub_inputs.has_mod {
            degrees[RANGE_RECON_CON] = cyclic(1);
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
        let two64 = E::from(Felt::from(u64::MAX)) + E::ONE;
        let mut wrap_bit_sum = E::ZERO;
        for i in 0..NUM_RANGE_BITS {
            wrap_bit_sum += E::from(Felt::from(1u64 << i)) * next_row[WRAP_BITS_BASE + i];
        }

        // all reg except dest should not change. dest should be next_res
        for j in 0..NUM_REGISTERS {
            result[j] = (next_row[j] - curr_row[j])
                - curr_pub_in[P_RES_BASE + j] * (next_res - curr_row[j]);
        }

        let is_add = curr_pub_in[P_IS_ADD];
        let is_sub = curr_pub_in[P_IS_SUB];
        let is_mul = curr_pub_in[P_IS_MUL];
        let is_lt = curr_pub_in[P_IS_LT];
        let is_mod = curr_pub_in[P_IS_MOD];

        result[RES_COL] = curr_pub_in[P_IS_SET] * (next_res - curr_pub_in[P_CONST])
            + is_add * (next_res + wrap_bit_sum * two64 - next_src1 - next_src2)
            + is_sub * (next_res - next_src1 + next_src2 - wrap_bit_sum * two64)
            + is_mul * (next_res + wrap_bit_sum * two64 - next_src1 * next_src2)
            + curr_pub_in[P_IS_ASSERT_EQ] * (next_res - E::ONE);

        // next[src1/2] should be the dot product of their reg selectors and curr regs
        let (mut exp_s1, mut exp_s2) = (E::ZERO, E::ZERO);
        for j in 0..NUM_REGISTERS {
            exp_s1 += curr_pub_in[P_SRC1_BASE + j] * curr_row[j];
            exp_s2 += curr_pub_in[P_SRC2_BASE + j] * curr_row[j];
        }
        result[SRC1_COL] = next_src1 - exp_s1;
        result[SRC2_COL] = next_src2 - exp_s2;
        result[ASSERT_EQ_CON] =
            curr_pub_in[P_IS_ASSERT_EQ] * (next_src1 - next_src2 - (next_res - E::ONE));
        if self.public_inputs.has_mod {
            result[QUOT_COL] = (E::ONE - is_mod) * (next_quot - E::ONE);
            result[MOD_REL_CON] =
                is_mod * (next_src1 - (next_src2 * (next_quot - E::ONE) + next_res));
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
        result[LT_RES_BOOL_CON] = is_lt * next_res * (next_res - E::ONE);

        // combined reconstruction. lt rows decompose comparison diff, mod rows decompose src2-res-1,
        // and all other rows decompose res.
        let exp_diff = next_res * (next_src2 - next_src1 - E::ONE)
            + (E::ONE - next_res) * (next_src1 - next_src2);
        let mod_diff = next_src2 - next_res - E::ONE;
        let mut bit_sum = E::ZERO;
        for i in 0..NUM_RANGE_BITS {
            bit_sum += E::from(Felt::from(1u64 << i)) * next_row[RANGE_BITS_BASE + i];
        }
        result[RANGE_RECON_CON] = is_lt * (exp_diff - bit_sum)
            + is_mod * (mod_diff - bit_sum)
            + (E::ONE - is_lt - is_mod) * (next_res - bit_sum);
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
