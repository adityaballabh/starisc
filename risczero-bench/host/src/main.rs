use methods::{FIB_ELF, FIB_ID};
use risc0_zkvm::{default_prover, ExecutorEnv};
use std::fs;
use std::path::Path;
use std::time::Instant;

fn bench(n: u32) -> String {
    let env = ExecutorEnv::builder().write(&n).unwrap().build().unwrap();

    let prover = default_prover();

    let t_proof = Instant::now();
    let proof_info = prover.prove(env, FIB_ELF).unwrap();
    let proof_ms = t_proof.elapsed().as_secs_f64() * 1000.0;

    let receipt = proof_info.receipt;
    let proof_size = bincode::serialize(&receipt).unwrap().len();

    let t_verify = Instant::now();
    receipt.verify(FIB_ID).unwrap();
    let verify_ms = t_verify.elapsed().as_secs_f64() * 1000.0;

    format!(
        "fib_{}: prove={:.3}ms  verify={:.3}ms  proof={}KB",
        n,
        proof_ms,
        verify_ms,
        proof_size / 1024
    )
}

fn main() {
    let res_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("results");
    fs::create_dir_all(&res_dir).unwrap();

    for n in [8, 16] {
        let res = bench(n);
        println!("{}", res);
        fs::write(res_dir.join(format!("fib_{}.txt", n)), &res).unwrap();
    }
}
