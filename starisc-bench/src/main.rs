use std::fs::{self, OpenOptions};
use std::io::Write as IoWrite;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

const RUNS: usize = 5;

fn project_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn compile(path: &Path) -> String {
    let status = Command::new("python3")
        .args(["-m", "compiler", path.to_str().unwrap()])
        .current_dir(project_root())
        .status()
        .unwrap();

    assert!(status.success(), "compiler failed on {:?}", path);
    format!("{}.op", path.with_extension("").display())
}

fn load_extra_args(py_path: &Path) -> Vec<String> {
    let args_path = py_path.with_extension("args");
    match fs::read_to_string(&args_path) {
        Ok(content) => content.split_whitespace().map(String::from).collect(),
        Err(_) => vec![],
    }
}

fn starisc_bin() -> std::path::PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    path.push("starisc");
    path
}

fn bench_once(
    name: &str,
    op_path: &str,
    extra_args: &[String],
    log_dir: &Path,
) -> (f64, f64, usize) {
    let proof_path = log_dir.join(format!("{}.proof", name));
    let bin = starisc_bin();

    let mut prove_cmd = Command::new(&bin);
    prove_cmd.arg("prove").arg(op_path);
    prove_cmd.args(extra_args);
    prove_cmd.args(["--output", proof_path.to_str().unwrap()]);

    let t_proof = Instant::now();
    let output = prove_cmd.output().unwrap();
    let proof_ms = t_proof.elapsed().as_secs_f64() * 1000.0;

    assert!(
        output.status.success(),
        "prove failed for {}: {}",
        name,
        String::from_utf8_lossy(&output.stderr)
    );

    let proof_kb = fs::metadata(&proof_path).unwrap().len() as usize / 1024;

    let mut verify_extra = vec![];
    let mut i = 0;
    while i < extra_args.len() {
        if extra_args[i] == "--private" {
            i += 2;
            continue;
        }
        verify_extra.push(extra_args[i].clone());
        i += 1;
    }

    let mut verify_cmd = Command::new(&bin);
    verify_cmd.arg("verify").arg(op_path);
    verify_cmd.args(["--proof", proof_path.to_str().unwrap()]);
    verify_cmd.args(&verify_extra);

    let t_verify = Instant::now();
    let output = verify_cmd.output().unwrap();
    let verify_ms = t_verify.elapsed().as_secs_f64() * 1000.0;

    assert!(
        output.status.success(),
        "verify failed for {}: {}",
        name,
        String::from_utf8_lossy(&output.stderr)
    );

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
        .filter(|path| {
            path.extension().is_some_and(|ext| ext == "py")
                && path.file_stem().unwrap() != "starisc"
        })
        .collect();

    let log_dir = bench_dir.join("logs");
    fs::create_dir_all(&log_dir).unwrap();

    for path in paths {
        let name = path.file_stem().unwrap().to_str().unwrap().to_owned();
        let op_path = compile(&path);
        let extra_args = load_extra_args(&path);
        let out_path = res_dir.join(format!("{}.txt", name));

        fs::write(&out_path, "").unwrap();

        let mut totals = (0.0_f64, 0.0_f64, 0_usize);
        for run in 1..=RUNS {
            let (prove_ms, verify_ms, proof_kb) =
                bench_once(&name, &op_path, &extra_args, &log_dir);
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
    }
}
