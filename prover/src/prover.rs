use crate::air::VmAir;
use crate::public_inputs::{PublicInputFlags, PublicInputs};
use crate::trace_builder::{build_trace, get_trace_len};
use crate::Felt;
use vm::{Instruction, Trace};
use winterfell::crypto::hashers::Blake3_256;
use winterfell::crypto::{DefaultRandomCoin, MerkleTree};
use winterfell::math::fields::f128::BaseElement;
use winterfell::matrix::ColMatrix;
use winterfell::{
    AcceptableOptions, DefaultConstraintCommitment, DefaultConstraintEvaluator, DefaultTraceLde,
    Proof, ProofOptions, Prover, ProverError, StarkDomain, TraceInfo, TracePolyTable, TraceTable,
    VerifierError,
};

const EXEC_ERR: &str = "program failed to execute";

// winterfell defaults (docs: 96-bit security with 32 queries). 64-bit VM -> 22 queries * log2(8) = 66-bit security
const NUM_QUERIES: usize = 22;
const BLOWUP_FACTOR: usize = 8;
const GRINDING_FACTOR: u32 = 0;
const FRI_FOLDING_FACTOR: usize = 8;
const FRI_REMAINDER_MAX_DEGREE: usize = 31;
// verifier rejects proofs below this security level
const MIN_VERIFY_SECURITY_BITS: u32 = 64;
const CLAIM_SCRATCH_REGISTER: u8 = 15;
const CLAIM_FALLBACK_SCRATCH_REGISTER: u8 = 14;

pub(crate) struct VmProver {
    options: ProofOptions,
    pub_inputs: PublicInputs,
}

impl VmProver {
    pub fn new(
        prog: &[Instruction],
        public_inputs: &[u64],
        bits_used: u64,
        wrap_bits_used: u64,
        flags: PublicInputFlags,
    ) -> Self {
        let trace_len = get_trace_len(prog);
        let options = ProofOptions::new(
            NUM_QUERIES,
            BLOWUP_FACTOR,
            GRINDING_FACTOR,
            winterfell::FieldExtension::None,
            FRI_FOLDING_FACTOR,
            FRI_REMAINDER_MAX_DEGREE,
        );
        let pub_inputs = PublicInputs::new(
            prog.to_vec(),
            public_inputs.to_vec(),
            trace_len,
            bits_used,
            wrap_bits_used,
            flags,
        );
        Self {
            options,
            pub_inputs,
        }
    }
}

impl Prover for VmProver {
    type BaseField = Felt;
    type Air = VmAir;
    type Trace = TraceTable<Felt>;
    type HashFn = Blake3_256<BaseElement>;
    type VC = MerkleTree<Self::HashFn>;
    type RandomCoin = DefaultRandomCoin<Self::HashFn>;
    type TraceLde<E: winterfell::math::FieldElement<BaseField = Felt>> =
        DefaultTraceLde<E, Self::HashFn, Self::VC>;
    type ConstraintEvaluator<'a, E: winterfell::math::FieldElement<BaseField = Felt>> =
        DefaultConstraintEvaluator<'a, VmAir, E>;
    type ConstraintCommitment<E: winterfell::math::FieldElement<BaseField = Felt>> =
        DefaultConstraintCommitment<E, Self::HashFn, Self::VC>;

    fn get_pub_inputs(&self, _trace: &Self::Trace) -> PublicInputs {
        self.pub_inputs.clone()
    }

    fn options(&self) -> &ProofOptions {
        &self.options
    }

    fn new_trace_lde<E: winterfell::math::FieldElement<BaseField = Felt>>(
        &self,
        trace_info: &TraceInfo,
        main_trace: &ColMatrix<Felt>,
        domain: &StarkDomain<Felt>,
        partition_option: winterfell::PartitionOptions,
    ) -> (Self::TraceLde<E>, TracePolyTable<E>) {
        DefaultTraceLde::new(trace_info, main_trace, domain, partition_option)
    }

    fn new_evaluator<'a, E: winterfell::math::FieldElement<BaseField = Felt>>(
        &self,
        air: &'a VmAir,
        aux_rand_elements: Option<winterfell::AuxRandElements<E>>,
        composition_coefficients: winterfell::ConstraintCompositionCoefficients<E>,
    ) -> Self::ConstraintEvaluator<'a, E> {
        DefaultConstraintEvaluator::new(air, aux_rand_elements, composition_coefficients)
    }

    fn build_constraint_commitment<E: winterfell::math::FieldElement<BaseField = Felt>>(
        &self,
        composition_poly_trace: winterfell::CompositionPolyTrace<E>,
        num_constraint_composition_columns: usize,
        domain: &StarkDomain<Felt>,
        partition_options: winterfell::PartitionOptions,
    ) -> (
        Self::ConstraintCommitment<E>,
        winterfell::CompositionPoly<E>,
    ) {
        DefaultConstraintCommitment::new(
            composition_poly_trace,
            num_constraint_composition_columns,
            domain,
            partition_options,
        )
    }
}

pub fn prove(prog: &[Instruction], vm_trace: &Trace) -> Result<Proof, ProverError> {
    let (bits_used, wrap_bits_used) = conservative_bits(prog);
    let prover = VmProver::new(
        prog,
        &[],
        bits_used,
        wrap_bits_used,
        conservative_flags(prog),
    );
    let trace = build_trace(prog, vm_trace);
    prover.prove(trace)
}

fn conservative_flags(prog: &[Instruction]) -> PublicInputFlags {
    let uses_src1 = prog.iter().any(|instr| {
        matches!(
            instr,
            Instruction::Add { .. }
                | Instruction::Sub { .. }
                | Instruction::Mul { .. }
                | Instruction::AssertEq { .. }
                | Instruction::Lt { .. }
                | Instruction::Mod { .. }
                | Instruction::Jz { .. }
        )
    });
    let uses_src2 = prog.iter().any(|instr| {
        matches!(
            instr,
            Instruction::Add { .. }
                | Instruction::Sub { .. }
                | Instruction::Mul { .. }
                | Instruction::AssertEq { .. }
                | Instruction::Lt { .. }
                | Instruction::Mod { .. }
        )
    });
    let has_jz = prog
        .iter()
        .any(|instr| matches!(instr, Instruction::Jz { .. }));

    PublicInputFlags {
        has_nonzero_src1: uses_src1,
        has_nonzero_src2: uses_src2,
        has_taken_jz: has_jz,
        has_not_taken_jz: has_jz,
    }
}

fn conservative_bits(prog: &[Instruction]) -> (u64, u64) {
    let _ = prog;
    (u64::MAX, u64::MAX)
}

pub fn prove_with_inputs(
    prog: &[Instruction],
    private_inputs: &[u64],
    public_inputs: &[u64],
) -> Result<Proof, ProverError> {
    let vm_trace = vm::execute_with_inputs(prog, private_inputs, public_inputs)
        .expect(EXEC_ERR)
        .0;
    let (bits_used, wrap_bits_used) = conservative_bits(prog);
    let flags = conservative_flags(prog);
    let prover = VmProver::new(prog, public_inputs, bits_used, wrap_bits_used, flags);
    let trace = build_trace(prog, &vm_trace);
    prover.prove(trace)
}

pub fn verify(prog: &[Instruction], proof: Proof) -> Result<(), VerifierError> {
    verify_with_inputs(prog, proof, &[])
}

pub fn verify_with_inputs(
    prog: &[Instruction],
    proof: Proof,
    public_inputs: &[u64],
) -> Result<(), VerifierError> {
    let trace_len = get_trace_len(prog);
    let (bits_used, wrap_bits_used) = conservative_bits(prog);
    let flags = conservative_flags(prog);
    let pub_inputs = PublicInputs::new(
        prog.to_vec(),
        public_inputs.to_vec(),
        trace_len,
        bits_used,
        wrap_bits_used,
        flags,
    );
    let min_proof_bits = AcceptableOptions::MinConjecturedSecurity(MIN_VERIFY_SECURITY_BITS);
    winterfell::verify::<
        VmAir,
        Blake3_256<BaseElement>,
        DefaultRandomCoin<Blake3_256<BaseElement>>,
        MerkleTree<Blake3_256<BaseElement>>,
    >(proof, pub_inputs, &min_proof_bits)
}

#[derive(Debug, Clone, Copy)]
pub struct Claim {
    pub register: u8,
    pub expected: u64,
}

pub fn extend_with_claim(prog: &[Instruction], claim: &Claim) -> Vec<Instruction> {
    let scratch = if claim.register == CLAIM_SCRATCH_REGISTER {
        CLAIM_FALLBACK_SCRATCH_REGISTER
    } else {
        CLAIM_SCRATCH_REGISTER
    };
    let mut extended = prog.to_vec();
    extended.push(Instruction::Set {
        dest: scratch,
        val: claim.expected,
    });
    extended.push(Instruction::AssertEq {
        r1: claim.register,
        r2: scratch,
    });
    extended
}

pub fn prove_with_claim(
    prog: &[Instruction],
    private_inputs: &[u64],
    public_inputs: &[u64],
    claim: &Claim,
) -> Result<Proof, ProverError> {
    let extended = extend_with_claim(prog, claim);
    prove_with_inputs(&extended, private_inputs, public_inputs)
}

pub fn verify_with_claim(
    prog: &[Instruction],
    proof: Proof,
    public_inputs: &[u64],
    claim: &Claim,
) -> Result<(), VerifierError> {
    let extended = extend_with_claim(prog, claim);
    verify_with_inputs(&extended, proof, public_inputs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace_builder::{build_trace, get_trace_len};
    use crate::{MOD_RES_BITS_BASE, QUOT_COL, RANGE_BITS_BASE, RES_COL, WRAP_BITS_BASE};
    use test_utils::{assert_proof_rejected, get_op_path};
    use vm::{parse_file, parse_str};
    use winterfell::math::FieldElement;

    const MOD_DEST: usize = 3;
    const MOD_TRACE_ROW: usize = 3;

    fn set_mod_result(trace: &mut TraceTable<Felt>, trace_len: usize, value: Felt) {
        trace.set(RES_COL, MOD_TRACE_ROW, value);
        for row in MOD_TRACE_ROW..trace_len {
            trace.set(MOD_DEST, row, value);
        }
    }

    fn assert_malformed_trace_rejected(prog: Vec<Instruction>, trace: TraceTable<Felt>) {
        let (bits_used, wrap_bits_used) = conservative_bits(&prog);
        let prover = VmProver::new(
            &prog,
            &[],
            bits_used,
            wrap_bits_used,
            conservative_flags(&prog),
        );
        let prog_clone = prog.clone();
        assert_proof_rejected(
            move || prover.prove(trace),
            |proof| verify(&prog_clone, proof),
        );
    }

    /// malicious prover trying to inject a value > u64::MAX into the trace
    #[test]
    fn rejects_overflow_injection() {
        let prog = parse_file(&get_op_path("limited_ops")).unwrap();
        let (vm_trace, _) = vm::execute(&prog).unwrap();
        let mut trace = build_trace(&prog, &vm_trace);
        trace.set(RES_COL, 3, Felt::from(u64::MAX) + Felt::ONE);
        let (bits_used, wrap_bits_used) = conservative_bits(&prog);

        let prover = VmProver::new(
            &prog,
            &[],
            bits_used,
            wrap_bits_used,
            conservative_flags(&prog),
        );
        let (prog_clone, trace_clone) = (prog.clone(), trace);
        assert_proof_rejected(
            move || prover.prove(trace_clone),
            |proof| verify(&prog_clone, proof),
        );
    }

    #[test]
    fn rejects_non_integer_mod_quotient() {
        let prog = parse_str("SET r1 5\nSET r2 3\nMOD r3 r1 r2").unwrap();
        let (vm_trace, _) = vm::execute(&prog).unwrap();
        let trace_len = get_trace_len(&prog);
        let mut trace = build_trace(&prog, &vm_trace);

        set_mod_result(&mut trace, trace_len, Felt::ZERO);
        trace.set(
            QUOT_COL,
            MOD_TRACE_ROW,
            Felt::from(5u64) * Felt::from(3u64).inv() + Felt::ONE,
        );
        trace.set(RANGE_BITS_BASE + 1, MOD_TRACE_ROW, Felt::ONE);
        trace.set(MOD_RES_BITS_BASE + 1, MOD_TRACE_ROW, Felt::ZERO);

        assert_malformed_trace_rejected(prog, trace);
    }

    /// Integer q alone is insufficient: q = 2 gives a field-wrapped remainder of -1.
    #[test]
    fn rejects_field_wrapped_mod_remainder() {
        let prog = parse_str("SET r1 5\nSET r2 3\nMOD r3 r1 r2").unwrap();
        let (vm_trace, _) = vm::execute(&prog).unwrap();
        let trace_len = get_trace_len(&prog);
        let mut trace = build_trace(&prog, &vm_trace);

        set_mod_result(&mut trace, trace_len, Felt::ZERO - Felt::ONE);
        trace.set(QUOT_COL, MOD_TRACE_ROW, Felt::from(3u64));
        trace.set(WRAP_BITS_BASE, MOD_TRACE_ROW, Felt::ZERO);
        trace.set(WRAP_BITS_BASE + 1, MOD_TRACE_ROW, Felt::ONE);
        trace.set(RANGE_BITS_BASE, MOD_TRACE_ROW, Felt::ONE);
        trace.set(RANGE_BITS_BASE + 1, MOD_TRACE_ROW, Felt::ONE);

        assert_malformed_trace_rejected(prog, trace);
    }

    #[test]
    fn rejects_out_of_range_private_read() {
        let prog = parse_str("READ_PRIV r1 0").unwrap();
        let (vm_trace, _) = vm::execute_with_inputs(&prog, &[0], &[]).unwrap();
        let trace_len = get_trace_len(&prog);
        let mut trace = build_trace(&prog, &vm_trace);
        let out_of_range = Felt::from(u64::MAX) + Felt::ONE;

        trace.set(RES_COL, 1, out_of_range);
        for row in 1..trace_len {
            trace.set(1, row, out_of_range);
        }

        assert_malformed_trace_rejected(prog, trace);
    }
}
