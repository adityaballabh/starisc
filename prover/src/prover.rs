use crate::air::VmAir;
use crate::public_inputs::PublicInputs;
use crate::trace_builder::{build_trace, get_bits_used, get_trace_len};
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

pub(crate) struct VmProver {
    options: ProofOptions,
    pub_inputs: PublicInputs,
}

impl VmProver {
    pub fn new(prog: &[Instruction], bits_used: u64) -> Self {
        let trace_len = get_trace_len(prog);
        let options = ProofOptions::new(
            NUM_QUERIES,
            BLOWUP_FACTOR,
            GRINDING_FACTOR,
            winterfell::FieldExtension::None,
            FRI_FOLDING_FACTOR,
            FRI_REMAINDER_MAX_DEGREE,
        );
        let pub_inputs = PublicInputs::new(prog.to_vec(), trace_len, bits_used);
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
    let prover = VmProver::new(prog, get_bits_used(prog, vm_trace));
    let trace = build_trace(prog, vm_trace);
    prover.prove(trace)
}

pub fn verify(prog: &[Instruction], proof: Proof) -> Result<(), VerifierError> {
    let trace_len = get_trace_len(prog);
    let vm_trace = &vm::execute(prog).expect(EXEC_ERR).0;
    let pub_inputs = PublicInputs::new(prog.to_vec(), trace_len, get_bits_used(prog, vm_trace));
    let min_proof_bits = AcceptableOptions::MinConjecturedSecurity(MIN_VERIFY_SECURITY_BITS);
    winterfell::verify::<
        VmAir,
        Blake3_256<BaseElement>,
        DefaultRandomCoin<Blake3_256<BaseElement>>,
        MerkleTree<Blake3_256<BaseElement>>,
    >(proof, pub_inputs, &min_proof_bits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace_builder::{build_trace, get_bits_used};
    use crate::RES_COL;
    use test_utils::{assert_proof_rejected, get_op_path};
    use vm::parse_file;
    use winterfell::math::FieldElement;

    /// malicious prover trying to inject a value > u64::MAX into the trace
    #[test]
    fn rejects_overflow_injection() {
        let prog = parse_file(&get_op_path("limited_ops")).unwrap();
        let (vm_trace, _) = vm::execute(&prog).unwrap();
        let mut trace = build_trace(&prog, &vm_trace);
        trace.set(RES_COL, 3, Felt::from(u64::MAX) + Felt::ONE);

        let prover = VmProver::new(&prog, get_bits_used(&prog, &vm_trace));
        let (prog_clone, trace_clone) = (prog.clone(), trace);
        assert_proof_rejected(
            move || prover.prove(trace_clone),
            |proof| verify(&prog_clone, proof),
        );
    }
}
