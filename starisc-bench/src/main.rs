use std::fs::{self, OpenOptions};
use std::io::Write as IoWrite;
use std::path::Path;
use std::process::Command;

const RUNS: usize = 5;
const PYTHON: &str = "python3";
const COMPILER: &str = "compiler";
const STARISC: &str = "starisc";
const PROVE: &str = "prove";
const VERIFY: &str = "verify";
const OUT_DIR: &str = "--out-dir";
const OUTPUT: &str = "--output";
const PROOF_FLAG: &str = "--proof";
const PRIVATE: &str = "--private";
const AIR_OUTPUT: &str = "--air-output";
const PROVE_MS: &str = "prove_ms=";
const VERIFY_MS: &str = "verify_ms=";
const PY_EXT: &str = "py";
const OP_EXT: &str = "op";
const ARGS_EXT: &str = "args";
const PROOF_EXT: &str = "proof";
const TXT_EXT: &str = "txt";
const AIR_EXT: &str = "air.txt";
const PROGRAMS: &str = "programs";
const GENERATED: &str = "generated";
const RESULTS: &str = "results";
const LOGS: &str = "logs";
const SUPPORT_MOD: &str = "starisc";

fn project_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn compile(path: &Path, out_dir: &Path) -> String {
    let status = Command::new(PYTHON)
        .args([
            "-m",
            COMPILER,
            path.to_str().unwrap(),
            OUT_DIR,
            out_dir.to_str().unwrap(),
        ])
        .current_dir(project_root())
        .status()
        .unwrap();

    assert!(status.success(), "compiler failed on {:?}", path);
    out_dir
        .join(format!(
            "{}.{OP_EXT}",
            path.file_stem().unwrap().to_str().unwrap()
        ))
        .to_string_lossy()
        .into_owned()
}

fn load_extra_args(py_path: &Path) -> Vec<String> {
    let args_path = py_path.with_extension(ARGS_EXT);
    match fs::read_to_string(&args_path) {
        Ok(content) => content.split_whitespace().map(String::from).collect(),
        Err(_) => vec![],
    }
}

fn starisc_bin() -> std::path::PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    path.push(STARISC);
    path
}

fn parse_elapsed_ms(stdout: &[u8], label: &str) -> f64 {
    String::from_utf8_lossy(stdout)
        .lines()
        .find_map(|line| line.strip_prefix(label))
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| panic!("missing {label} in starisc output"))
}

fn bench_once(
    name: &str,
    op_path: &str,
    extra_args: &[String],
    log_dir: &Path,
) -> (f64, f64, usize) {
    let proof_path = log_dir.join(format!("{}.{PROOF_EXT}", name));
    let bin = starisc_bin();

    let mut prove_cmd = Command::new(&bin);
    prove_cmd.arg(PROVE).arg(op_path);
    prove_cmd.args(extra_args);
    prove_cmd.args([OUTPUT, proof_path.to_str().unwrap()]);

    let output = prove_cmd.output().unwrap();

    assert!(
        output.status.success(),
        "prove failed for {}: {}",
        name,
        String::from_utf8_lossy(&output.stderr)
    );

    let proof_ms = parse_elapsed_ms(&output.stdout, PROVE_MS);
    let proof_kb = fs::metadata(&proof_path).unwrap().len() as usize / 1024;

    let mut verify_extra = vec![];
    let mut i = 0;
    while i < extra_args.len() {
        if extra_args[i] == PRIVATE {
            i += 2;
            continue;
        }
        verify_extra.push(extra_args[i].clone());
        i += 1;
    }

    let mut verify_cmd = Command::new(&bin);
    verify_cmd.arg(VERIFY).arg(op_path);
    verify_cmd.args([PROOF_FLAG, proof_path.to_str().unwrap()]);
    verify_cmd.args(&verify_extra);

    let output = verify_cmd.output().unwrap();

    assert!(
        output.status.success(),
        "verify failed for {}: {}",
        name,
        String::from_utf8_lossy(&output.stderr)
    );

    let verify_ms = parse_elapsed_ms(&output.stdout, VERIFY_MS);

    (proof_ms, verify_ms, proof_kb)
}

fn write_air_log(name: &str, op_path: &str, extra_args: &[String], log_dir: &Path) {
    let proof_path = log_dir.join(format!("{}.{PROOF_EXT}", name));
    let air_path = log_dir.join(format!("{}.{AIR_EXT}", name));
    let bin = starisc_bin();

    let mut cmd = Command::new(&bin);
    cmd.arg(PROVE).arg(op_path);
    cmd.args(extra_args);
    cmd.args([OUTPUT, proof_path.to_str().unwrap()]);
    cmd.args([AIR_OUTPUT, air_path.to_str().unwrap()]);

    let output = cmd.output().unwrap();

    assert!(
        output.status.success(),
        "air output failed for {}: {}",
        name,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn main() {
    let write_air_output = std::env::args().skip(1).any(|arg| arg == AIR_OUTPUT);
    let bench_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let prog_dir = bench_dir.join(PROGRAMS);
    let generated_dir = bench_dir.join(GENERATED);
    let res_dir = bench_dir.join(RESULTS);
    fs::create_dir_all(&generated_dir).unwrap();
    fs::create_dir_all(&res_dir).unwrap();

    let paths: Vec<_> = fs::read_dir(&prog_dir)
        .unwrap()
        .flatten()
        .map(|file| file.path())
        .filter(|path| {
            path.extension().is_some_and(|ext| ext == PY_EXT)
                && path.file_stem().unwrap() != SUPPORT_MOD
        })
        .collect();

    let log_dir = bench_dir.join(LOGS);
    fs::create_dir_all(&log_dir).unwrap();

    for path in paths {
        let name = path.file_stem().unwrap().to_str().unwrap().to_owned();
        let op_path = compile(&path, &generated_dir);
        let extra_args = load_extra_args(&path);
        let out_path = res_dir.join(format!("{}.{TXT_EXT}", name));

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

        if write_air_output {
            write_air_log(&name, &op_path, &extra_args, &log_dir);
        }
    }
}
