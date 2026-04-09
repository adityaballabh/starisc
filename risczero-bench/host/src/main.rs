use methods::{FIB_ELF, FIB_ID, RSA_64_ELF, RSA_64_ID};
use risc0_zkvm::{default_prover, ExecutorEnv};
use std::fs;
use std::path::Path;
use std::time::Instant;

fn bench<T: serde::Serialize>(name: &str, input: &T, elf: &[u8], id: [u32; 8], res_dir: &Path) {
    let env = ExecutorEnv::builder()
        .write(input)
        .unwrap()
        .build()
        .unwrap();

    let prover = default_prover();

    let t_proof = Instant::now();
    let proof_info = prover.prove(env, elf).unwrap();
    let proof_ms = t_proof.elapsed().as_secs_f64() * 1000.0;

    let receipt = proof_info.receipt;
    let proof_size = bincode::serialize(&receipt).unwrap().len();

    let t_verify = Instant::now();
    receipt.verify(id).unwrap();
    let verify_ms = t_verify.elapsed().as_secs_f64() * 1000.0;

    let res = format!(
        "{}: prove={:.3}ms  verify={:.3}ms  proof={}KB",
        name,
        proof_ms,
        verify_ms,
        proof_size / 1024
    );
    println!("{}", res);
    fs::write(res_dir.join(format!("{}.txt", name)), &res).unwrap();
}

fn bench_fib(res_dir: &Path) {
    let inputs: [u32; 2] = [8, 16];
    for n in inputs {
        bench(&format!("fib_{}", n), &n, FIB_ELF, FIB_ID, res_dir);
    }
}

fn bench_rsa_64(res_dir: &Path) {
    bench("rsa_64", &(), RSA_64_ELF, RSA_64_ID, res_dir);
}

fn main() {
    let res_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("results");
    fs::create_dir_all(&res_dir).unwrap();

    bench_fib(&res_dir);
    bench_rsa_64(&res_dir);
}
