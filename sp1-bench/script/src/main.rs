use sp1_sdk::{
    blocking::{ProveRequest, Prover, ProverClient},
    include_elf, Elf, ProvingKey, SP1Stdin,
};
use std::fs::{self, OpenOptions};
use std::io::Write as IoWrite;
use std::path::Path;
use std::time::Instant;

const RUNS: usize = 5;
const FIB_ELF: Elf = include_elf!("fib-program");
const RSA_ELF: Elf = include_elf!("rsa-program");

fn bench_once<P: Prover>(client: &P, pk: &P::ProvingKey, stdin: SP1Stdin) -> (f64, f64, usize) {
    let t_prove = Instant::now();
    let proof = client.prove(pk, stdin).run().unwrap();
    let prove_ms = t_prove.elapsed().as_secs_f64() * 1000.0;
    let proof_kb = bincode::serialize(&proof).unwrap().len() / 1024;

    let t_verify = Instant::now();
    client.verify(&proof, pk.verifying_key(), None).unwrap();
    let verify_ms = t_verify.elapsed().as_secs_f64() * 1000.0;

    (prove_ms, verify_ms, proof_kb)
}

fn bench<P: Prover, F>(name: &str, client: &P, pk: &P::ProvingKey, res_dir: &Path, mk_stdin: F)
where
    F: Fn() -> SP1Stdin,
{
    let out_path = res_dir.join(format!("{}.txt", name));
    fs::write(&out_path, "").unwrap();

    let mut totals = (0.0_f64, 0.0_f64, 0_usize);
    for run in 1..=RUNS {
        let (prove_ms, verify_ms, proof_kb) = bench_once(client, pk, mk_stdin());
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

fn bench_fib<P: Prover>(client: &P, res_dir: &Path) {
    let pk = client.setup(FIB_ELF).expect("failed to setup fib elf");
    for n in [8_u32, 16_u32] {
        bench(&format!("fib_{}", n), client, &pk, res_dir, move || {
            let mut stdin = SP1Stdin::new();
            stdin.write(&n);
            stdin
        });
    }
}

fn bench_rsa<P: Prover>(client: &P, res_dir: &Path) {
    let pk = client.setup(RSA_ELF).expect("failed to setup rsa elf");
    bench("rsa_32", client, &pk, res_dir, SP1Stdin::new);
}

fn main() {
    sp1_sdk::utils::setup_logger();

    let res_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("results");
    fs::create_dir_all(&res_dir).unwrap();

    let client = ProverClient::from_env();

    bench_fib(&client, &res_dir);
    bench_rsa(&client, &res_dir);
}
