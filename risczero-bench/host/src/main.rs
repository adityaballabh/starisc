use methods::{FIB_ELF, FIB_ID, RSA_DEC_ELF, RSA_DEC_ID, RSA_ENC_ELF, RSA_ENC_ID};
use risc0_zkvm::{default_prover, ExecutorEnv};
use std::fs::{self, OpenOptions};
use std::io::Write as IoWrite;
use std::path::Path;
use std::time::Instant;

const RUNS: usize = 5;
const FIB: &str = "fib";
const RSA_ENC: &str = "rsa_enc";
const RSA_DEC: &str = "rsa_dec";
const FIB_CASES: &[(u32, u64)] = &[
    (8, 1_286),
    (32, 133_344_710),
    (128, 13_335_296_880_932_502_726),
    (512, 11_289_386_247_850_834_118),
];

fn family_results_path(res_dir: &Path, family: &str) -> std::path::PathBuf {
    res_dir.join(format!("{family}.txt"))
}

fn bench_once<T: serde::Serialize>(
    input: &T,
    elf: &[u8],
    id: [u32; 8],
    expected: u64,
) -> (f64, f64, usize) {
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

    let actual: u64 = receipt.journal.decode().unwrap();
    assert_eq!(actual, expected, "journal output mismatch for benchmark");

    (proof_ms, verify_ms, proof_kb)
}

fn bench<T: serde::Serialize>(
    name: &str,
    input: &T,
    elf: &[u8],
    id: [u32; 8],
    expected: u64,
    res_dir: &Path,
) {
    let out_path = res_dir;
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(out_path)
        .unwrap();
    f.write_all(format!("===== {name} =====\n").as_bytes())
        .unwrap();

    let mut totals = (0.0_f64, 0.0_f64, 0_usize);
    for run in 1..=RUNS {
        let (prove_ms, verify_ms, proof_kb) = bench_once(input, elf, id, expected);
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
    println!("{}: {}", name, avg);
    let mut f = OpenOptions::new().append(true).open(&out_path).unwrap();
    f.write_all(avg.as_bytes()).unwrap();
    f.write_all(b"\n").unwrap();
}

fn bench_fib(res_dir: &Path) {
    let res_dir = family_results_path(res_dir, FIB);
    let mut cases = FIB_CASES.to_vec();
    cases.sort_by_key(|&(n, _)| n);
    for (n, expected) in cases {
        bench(
            &format!("fib_{}", n),
            &(n, 23_u64, 47_u64),
            FIB_ELF,
            FIB_ID,
            expected,
            &res_dir,
        );
    }
}

fn bench_rsa(res_dir: &Path) {
    let message: u64 = 1_337;
    let n: u64 = 4_200_970_013;

    let encrypted: u64 = 864_554_256;
    bench(
        "rsa_enc",
        &(message, n),
        RSA_ENC_ELF,
        RSA_ENC_ID,
        encrypted,
        &family_results_path(res_dir, RSA_ENC),
    );

    let d: u64 = 2_145_513_473;
    bench(
        "rsa_dec",
        &(encrypted, n, d),
        RSA_DEC_ELF,
        RSA_DEC_ID,
        message,
        &family_results_path(res_dir, RSA_DEC),
    );
}

fn main() {
    let res_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("results");
    fs::create_dir_all(&res_dir).unwrap();
    for family in [FIB, RSA_ENC, RSA_DEC] {
        fs::write(family_results_path(&res_dir, family), "").unwrap();
    }

    bench_fib(&res_dir);
    bench_rsa(&res_dir);
}
