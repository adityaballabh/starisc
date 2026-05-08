use sp1_sdk::{
    blocking::{ProveRequest, Prover, ProverClient},
    include_elf, Elf, ProvingKey, SP1Stdin,
};
use std::convert::TryInto;
use std::fs::{self, OpenOptions};
use std::io::Write as IoWrite;
use std::path::Path;
use std::time::Instant;

const RUNS: usize = 5;
const FIB: &str = "fib";
const RSA_ENC: &str = "rsa_enc";
const RSA_DEC: &str = "rsa_dec";
const FIB_CASES: &[(u32, u64)] = &[(8, 1_286), (16, 60_419)];
const FIB_ELF: Elf = include_elf!("fib-program");
const RSA_ENC_ELF: Elf = include_elf!("rsa-enc-program");
const RSA_DEC_ELF: Elf = include_elf!("rsa-dec-program");

fn family_results_path(res_dir: &Path, family: &str) -> std::path::PathBuf {
    res_dir.join(format!("{family}.txt"))
}

fn read_committed_u64(bytes: &[u8]) -> u64 {
    let bytes: [u8; 8] = bytes.try_into().expect("expected one committed u64");
    u64::from_le_bytes(bytes)
}

fn bench_once<P: Prover>(
    client: &P,
    pk: &P::ProvingKey,
    stdin: SP1Stdin,
    expected: u64,
) -> (f64, f64, usize) {
    let t_prove = Instant::now();
    let proof = client.prove(pk, stdin).run().unwrap();
    let prove_ms = t_prove.elapsed().as_secs_f64() * 1000.0;
    let proof_kb = bincode::serialize(&proof).unwrap().len() / 1024;

    let t_verify = Instant::now();
    client.verify(&proof, pk.verifying_key(), None).unwrap();
    let verify_ms = t_verify.elapsed().as_secs_f64() * 1000.0;

    let actual = read_committed_u64(proof.public_values.as_slice());
    assert_eq!(actual, expected, "public output mismatch for benchmark");

    (prove_ms, verify_ms, proof_kb)
}

fn bench<P: Prover, F>(
    name: &str,
    client: &P,
    pk: &P::ProvingKey,
    res_dir: &Path,
    expected: u64,
    mk_stdin: F,
) where
    F: Fn() -> SP1Stdin,
{
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
        let (prove_ms, verify_ms, proof_kb) = bench_once(client, pk, mk_stdin(), expected);
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
    f.write_all(b"\n").unwrap();
}

fn bench_fib<P: Prover>(client: &P, res_dir: &Path) {
    let res_dir = family_results_path(res_dir, FIB);
    let pk = client.setup(FIB_ELF).expect("failed to setup fib elf");
    let mut cases = FIB_CASES.to_vec();
    cases.sort_by_key(|&(n, _)| n);
    for (n, expected) in cases {
        bench(
            &format!("fib_{}", n),
            client,
            &pk,
            &res_dir,
            expected,
            move || {
                let mut stdin = SP1Stdin::new();
                stdin.write(&n);
                stdin.write(&23_u64);
                stdin.write(&47_u64);
                stdin
            },
        );
    }
}

fn bench_rsa<P: Prover>(client: &P, res_dir: &Path) {
    let message: u64 = 1_337;
    let n: u64 = 4_200_970_013;
    let encrypted: u64 = 864_554_256;
    let d: u64 = 2_145_513_473;

    let pk_enc = client
        .setup(RSA_ENC_ELF)
        .expect("failed to setup rsa_enc elf");
    bench(
        "rsa_enc",
        client,
        &pk_enc,
        &family_results_path(res_dir, RSA_ENC),
        encrypted,
        || {
            let mut stdin = SP1Stdin::new();
            stdin.write(&message);
            stdin.write(&n);
            stdin
        },
    );

    let pk_dec = client
        .setup(RSA_DEC_ELF)
        .expect("failed to setup rsa_dec elf");
    bench(
        "rsa_dec",
        client,
        &pk_dec,
        &family_results_path(res_dir, RSA_DEC),
        message,
        || {
            let mut stdin = SP1Stdin::new();
            stdin.write(&encrypted);
            stdin.write(&n);
            stdin.write(&d);
            stdin
        },
    );
}

fn main() {
    sp1_sdk::utils::setup_logger();

    let res_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("results");
    fs::create_dir_all(&res_dir).unwrap();
    for family in [FIB, RSA_ENC, RSA_DEC] {
        fs::write(family_results_path(&res_dir, family), "").unwrap();
    }

    let client = ProverClient::from_env();

    bench_fib(&client, &res_dir);
    bench_rsa(&client, &res_dir);
}
