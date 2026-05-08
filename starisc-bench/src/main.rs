use std::fs::{self, OpenOptions};
use std::io::Write as IoWrite;
use std::path::Path;
use std::process::Command;

const RUNS: usize = 5;
const PYTHON: &str = "python3";
const CARGO: &str = "cargo";
const COMPILER: &str = "compiler";
const BUILD: &str = "build";
const RELEASE: &str = "--release";
const PACKAGE: &str = "-p";
const STARISC_CLI_PACKAGE: &str = "starisc-cli";
const STARISC: &str = "starisc";
const PROVE: &str = "prove";
const VERIFY: &str = "verify";
const OUT_DIR: &str = "--out-dir";
const OUTPUT: &str = "--output";
const PROOF_FLAG: &str = "--proof";
const PRIVATE: &str = "--private";
const AIR_OUTPUT: &str = "--air-output";
const TRACE_OUTPUT: &str = "--trace-output";
const PROVE_MS: &str = "prove_ms=";
const VERIFY_MS: &str = "verify_ms=";
const PY_EXT: &str = "py";
const OP_EXT: &str = "op";
const ARGS_EXT: &str = "args";
const PROOF_EXT: &str = "proof";
const TXT_EXT: &str = "txt";
const AIR_SUFFIX: &str = "_air.txt";
const TRACE_SUFFIX: &str = "_trace.txt";
const PROGRAMS: &str = "programs";
const GENERATED: &str = "generated";
const RESULTS: &str = "results";
const LOGS: &str = "logs";
const SUPPORT_MOD: &str = "starisc";
const PY_DIR: &str = "py";
const ARGS_DIR: &str = "args";

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

fn program_family(path: &Path, prog_dir: &Path) -> String {
    path.strip_prefix(prog_dir)
        .ok()
        .and_then(|relative| relative.components().next())
        .and_then(|component| match component {
            std::path::Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .unwrap_or_else(|| "misc".to_string())
}

fn program_size(path: &Path) -> u64 {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(|stem| stem.split('_').find_map(|part| part.parse().ok()))
        .unwrap_or(u64::MAX)
}

fn load_extra_args(py_path: &Path) -> Vec<String> {
    let args_path = if py_path
        .parent()
        .and_then(|parent| parent.file_name())
        .is_some_and(|name| name == PY_DIR)
    {
        py_path
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join(ARGS_DIR)
            .join(py_path.file_name().unwrap())
            .with_extension(ARGS_EXT)
    } else {
        py_path.with_extension(ARGS_EXT)
    };
    match fs::read_to_string(&args_path) {
        Ok(content) => content.split_whitespace().map(String::from).collect(),
        Err(_) => vec![],
    }
}

fn collect_programs(dir: &Path, paths: &mut Vec<std::path::PathBuf>) {
    for file in fs::read_dir(dir).unwrap().flatten() {
        let path = file.path();
        if path.is_dir() {
            collect_programs(&path, paths);
        } else if path.extension().is_some_and(|ext| ext == PY_EXT)
            && path.file_stem().unwrap() != SUPPORT_MOD
        {
            paths.push(path);
        }
    }
}

fn starisc_bin() -> std::path::PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    path.push(STARISC);
    path
}

fn build_starisc_cli() {
    let mut cmd = Command::new(CARGO);
    cmd.args([BUILD, PACKAGE, STARISC_CLI_PACKAGE]);
    if !cfg!(debug_assertions) {
        cmd.arg(RELEASE);
    }
    let status = cmd.current_dir(project_root()).status().unwrap();
    assert!(status.success(), "failed to build {STARISC_CLI_PACKAGE}");
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

fn write_output_logs(name: &str, op_path: &str, extra_args: &[String], log_dir: &Path) {
    let proof_path = log_dir.join(format!("{}.{PROOF_EXT}", name));
    let air_path = log_dir.join(format!("{name}{AIR_SUFFIX}"));
    let trace_path = log_dir.join(format!("{name}{TRACE_SUFFIX}"));
    let bin = starisc_bin();

    let mut cmd = Command::new(&bin);
    cmd.arg(PROVE).arg(op_path);
    cmd.args(extra_args);
    cmd.args([OUTPUT, proof_path.to_str().unwrap()]);
    cmd.args([AIR_OUTPUT, air_path.to_str().unwrap()]);
    cmd.args([TRACE_OUTPUT, trace_path.to_str().unwrap()]);

    let output = cmd.output().unwrap();

    assert!(
        output.status.success(),
        "output log failed for {}: {}",
        name,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn main() {
    build_starisc_cli();

    let should_write_output_logs = std::env::args()
        .skip(1)
        .any(|arg| arg == AIR_OUTPUT || arg == TRACE_OUTPUT);
    let bench_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let prog_dir = bench_dir.join(PROGRAMS);
    let generated_dir = bench_dir.join(GENERATED);
    let res_dir = bench_dir.join(RESULTS);
    fs::create_dir_all(&generated_dir).unwrap();
    fs::create_dir_all(&res_dir).unwrap();

    let mut paths = vec![];
    collect_programs(&prog_dir, &mut paths);
    paths.sort_by(|left, right| {
        program_family(left, &prog_dir)
            .cmp(&program_family(right, &prog_dir))
            .then_with(|| program_size(left).cmp(&program_size(right)))
            .then_with(|| left.cmp(right))
    });

    let mut families: Vec<_> = paths
        .iter()
        .map(|path| program_family(path, &prog_dir))
        .collect();
    families.sort();
    families.dedup();
    for family in &families {
        fs::write(res_dir.join(format!("{}.{TXT_EXT}", family)), "").unwrap();
    }

    let log_dir = bench_dir.join(LOGS);
    fs::create_dir_all(&log_dir).unwrap();

    for path in paths {
        let name = path.file_stem().unwrap().to_str().unwrap().to_owned();
        let family = program_family(&path, &prog_dir);
        let family_generated_dir = generated_dir.join(&family);
        let family_log_dir = log_dir.join(&family);
        fs::create_dir_all(&family_generated_dir).unwrap();
        fs::create_dir_all(&family_log_dir).unwrap();

        let op_path = compile(&path, &family_generated_dir);
        let extra_args = load_extra_args(&path);
        let out_path = res_dir.join(format!("{}.{TXT_EXT}", family));

        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&out_path)
            .unwrap();
        f.write_all(format!("===== {name} =====\n").as_bytes())
            .unwrap();

        let mut totals = (0.0_f64, 0.0_f64, 0_usize);
        for run in 1..=RUNS {
            let (prove_ms, verify_ms, proof_kb) =
                bench_once(&name, &op_path, &extra_args, &family_log_dir);
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

        if should_write_output_logs {
            write_output_logs(&name, &op_path, &extra_args, &family_log_dir);
        }
    }
}
