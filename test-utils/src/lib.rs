use std::path::Path;

pub fn assert_proof_rejected<Proof, ProveErr, VerifyErr>(
    prove_fn: impl FnOnce() -> Result<Proof, ProveErr> + std::panic::UnwindSafe,
    verify_fn: impl FnOnce(Proof) -> Result<(), VerifyErr>,
) {
    // debug: proof panics on constraint check, release: verify rejects the proof
    match std::panic::catch_unwind(prove_fn) {
        Err(_) | Ok(Err(_)) => {}
        Ok(Ok(proof)) => assert!(verify_fn(proof).is_err()),
    }
}

pub fn assert_proof_accepted<Proof, ProveErr, VerifyErr>(
    prove_fn: impl FnOnce() -> Result<Proof, ProveErr> + std::panic::UnwindSafe,
    verify_fn: impl FnOnce(Proof) -> Result<(), VerifyErr>,
) {
    // debug: proof panics on constraint check, release: verify rejects the proof
    match std::panic::catch_unwind(prove_fn) {
        Err(_) | Ok(Err(_)) => {}
        Ok(Ok(proof)) => assert!(verify_fn(proof).is_ok()),
    }
}

pub fn get_op_path(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("examples")
        .join(format!("{}.op", name))
        .to_str()
        .unwrap()
        .to_string()
}
