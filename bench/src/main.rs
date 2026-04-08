use prover::prover::{prove, verify};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Instant;
use vm::{execute, parse_file};

fn compile(path: &Path) -> String {
    let status = Command::new("python3")
        .args(["-m", "compiler", path.to_str().unwrap()])
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap())
        .status()
        .unwrap();

    assert!(status.success(), "compiler failed on {:?}", path);
    format!("{}.op", path.with_extension("").display())
}

fn run(name: &str, py_path: &Path) -> String {
    let op_path = compile(py_path);
    let prog = parse_file(&op_path).unwrap();
    let (trace, _) = execute(&prog).unwrap();

    let t_proof = Instant::now();
    let proof = prove(&prog, &trace).unwrap();
    let proof_ms = t_proof.elapsed().as_secs_f64() * 1000.0;
    let proof_size = proof.to_bytes().len();

    let t_verify = Instant::now();
    verify(&prog, proof).unwrap();
    let verify_ms = t_verify.elapsed().as_secs_f64() * 1000.0;

    format!(
        "{}: prove={:.3}ms  verify={:.3}ms  proof={}KB",
        name,
        proof_ms,
        verify_ms,
        proof_size / 1024
    )
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

    for path in paths {
        let name = path.file_stem().unwrap().to_str().unwrap().to_owned();
        let res = run(&name, &path);
        println!("{}", res);
        let out_path = res_dir.join(format!("{}.txt", name));
        fs::write(&out_path, &res).unwrap();
    }
}
