use prover::prover::{prove, verify};
use std::fs::{self, OpenOptions};
use std::io::Write as IoWrite;
use std::path::Path;
use std::process::Command;
use std::time::Instant;
use vm::{dump_trace, execute, parse_file};

const RUNS: usize = 5;

fn compile(path: &Path) -> String {
    let status = Command::new("python3")
        .args(["-m", "compiler", path.to_str().unwrap()])
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap())
        .status()
        .unwrap();

    assert!(status.success(), "compiler failed on {:?}", path);
    format!("{}.op", path.with_extension("").display())
}

fn bench_once(name: &str, op_path: &str, log_dir: &Path) -> (f64, f64, usize) {
    let prog = parse_file(op_path).unwrap();
    let (trace, regs) = execute(&prog).unwrap();

    let trace_path = log_dir.join(format!("{}.trace.txt", name));
    dump_trace(&prog, &trace, &regs, trace_path.to_str().unwrap()).unwrap();

    let t_proof = Instant::now();
    let proof = prove(&prog, &trace).unwrap();
    let proof_ms = t_proof.elapsed().as_secs_f64() * 1000.0;
    let proof_kb = proof.to_bytes().len() / 1024;

    let t_verify = Instant::now();
    verify(&prog, proof).unwrap();
    let verify_ms = t_verify.elapsed().as_secs_f64() * 1000.0;

    (proof_ms, verify_ms, proof_kb)
}

fn main() {
    let bench_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let prog_dir = bench_dir.join("programs");
    let res_dir = bench_dir.join("results");
    fs::create_dir_all(&res_dir).unwrap();

    let paths: Vec<_> = fs::read_dir(&prog_dir)
        .unwrap()
        .flatten()
        .map(|file| file.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "py"))
        .collect();

    let log_dir = bench_dir.join("logs");
    fs::create_dir_all(&log_dir).unwrap();

    for path in paths {
        let name = path.file_stem().unwrap().to_str().unwrap().to_owned();
        let op_path = compile(&path);
        let out_path = res_dir.join(format!("{}.txt", name));

        fs::write(&out_path, "").unwrap();

        let mut totals = (0.0_f64, 0.0_f64, 0_usize);
        for run in 1..=RUNS {
            let (prove_ms, verify_ms, proof_kb) = bench_once(&name, &op_path, &log_dir);
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
}
