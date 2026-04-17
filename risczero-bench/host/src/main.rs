use methods::{FIB_ELF, FIB_ID, RSA_64_ELF, RSA_64_ID};
use risc0_zkvm::{default_prover, ExecutorEnv};
use std::fs::{self, OpenOptions};
use std::io::Write as IoWrite;
use std::path::Path;
use std::time::Instant;

const RUNS: usize = 5;

fn bench_once<T: serde::Serialize>(input: &T, elf: &[u8], id: [u32; 8]) -> (f64, f64, usize) {
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
    let proof_kb = bincode::serialize(&receipt).unwrap().len() / 1024;

    let t_verify = Instant::now();
    receipt.verify(id).unwrap();
    let verify_ms = t_verify.elapsed().as_secs_f64() * 1000.0;

    (proof_ms, verify_ms, proof_kb)
}

fn bench<T: serde::Serialize>(name: &str, input: &T, elf: &[u8], id: [u32; 8], res_dir: &Path) {
    let out_path = res_dir.join(format!("{}.txt", name));
    fs::write(&out_path, "").unwrap();

    let mut totals = (0.0_f64, 0.0_f64, 0_usize);
    for run in 1..=RUNS {
        let (prove_ms, verify_ms, proof_kb) = bench_once(input, elf, id);
        totals.0 += prove_ms;
        totals.1 += verify_ms;
        totals.2 += proof_kb;

        let line = format!(
            "run {}: prove={:.3}ms  verify={:.3}ms  proof={}KB\n",
            run, prove_ms, verify_ms, proof_kb
        );
        print!("{}: {}", name, line);
        let mut f = OpenOptions::new().append(true).open(&out_path).unwrap();
        f.write_all(line.as_bytes()).unwrap();
    }

    let n = RUNS as f64;
    let avg = format!(
        "avg(5): prove={:.3}ms  verify={:.3}ms  proof={}KB\n",
        totals.0 / n,
        totals.1 / n,
        totals.2 / RUNS
    );
    print!("{}: {}\n", name, avg);
    let mut f = OpenOptions::new().append(true).open(&out_path).unwrap();
    f.write_all(avg.as_bytes()).unwrap();
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
