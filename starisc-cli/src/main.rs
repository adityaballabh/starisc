use clap::{Parser, Subcommand};
use prover::air_output::write_air_table;
use prover::prover::{
    extend_with_claim, prove_with_claim, prove_with_inputs, verify_with_claim, verify_with_inputs,
    Claim,
};
use std::collections::HashMap;
use std::path::Path;
use std::process;
use std::time::Instant;
use vm::parse_file;

#[derive(Parser)]
#[command(name = "starisc")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Prove {
        program: String,
        #[arg(long, value_delimiter = ',')]
        private: Vec<u64>,
        #[arg(long, value_delimiter = ',')]
        public: Vec<u64>,
        #[arg(long)]
        claim: Option<String>,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(long)]
        air_output: Option<String>,
    },
    Verify {
        program: String,
        #[arg(long)]
        proof: String,
        #[arg(long, value_delimiter = ',')]
        public: Vec<u64>,
        #[arg(long)]
        claim: Option<String>,
    },
}

fn load_symbols(op_path: &str) -> HashMap<String, u8> {
    let symbols_path = Path::new(op_path).with_extension("symbols");
    let mut map = HashMap::new();
    if let Ok(content) = std::fs::read_to_string(&symbols_path) {
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() == 2 {
                if let Some(reg_num) = parts[1]
                    .strip_prefix('r')
                    .and_then(|n| n.parse::<u8>().ok())
                {
                    map.insert(parts[0].to_string(), reg_num);
                }
            }
        }
    }
    map
}

fn resolve_claim(s: &str, symbols: &HashMap<String, u8>) -> Result<Claim, String> {
    let (name, val_str) = s
        .split_once('=')
        .ok_or("claim must be of the form <var>=<val>")?;
    let name = name.trim();
    let expected: u64 = val_str.trim().parse().map_err(|_| "invalid claim value")?;
    let register = symbols.get(name).copied().ok_or_else(|| {
        format!("unknown variable '{name}'; no .symbols file or variable not found")
    })?;
    Ok(Claim { register, expected })
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Prove {
            program,
            private,
            public,
            claim,
            output,
            air_output,
        } => {
            let prog = parse_file(&program).unwrap_or_else(|e| {
                eprintln!("parse error: {e}");
                process::exit(1);
            });

            let symbols = load_symbols(&program);
            let claim = claim.map(|c| {
                resolve_claim(&c, &symbols).unwrap_or_else(|e| {
                    eprintln!("invalid claim: {e}");
                    process::exit(1);
                })
            });

            if !private.is_empty() && claim.is_none() {
                eprintln!("error: private inputs require a --claim");
                process::exit(1);
            }

            let timer = Instant::now();
            let proof = match &claim {
                Some(c) => prove_with_claim(&prog, &private, &public, c),
                None => prove_with_inputs(&prog, &private, &public),
            };
            let prove_ms = timer.elapsed().as_secs_f64() * 1000.0;

            let proof = proof.unwrap_or_else(|e| {
                eprintln!("proving failed: {e}");
                process::exit(1);
            });

            let bytes = proof.to_bytes();
            let out_path = output.unwrap_or_else(|| format!("{}.proof", program));
            std::fs::write(&out_path, &bytes).unwrap_or_else(|e| {
                eprintln!("failed to write proof: {e}");
                process::exit(1);
            });

            if let Some(path) = air_output {
                let air_prog = match claim {
                    Some(claim) => extend_with_claim(&prog, &claim),
                    None => prog.clone(),
                };
                let (vm_trace, _) = vm::execute_with_inputs(&air_prog, &private, &public)
                    .unwrap_or_else(|e| {
                        eprintln!("failed to build AIR output trace: {e}");
                        process::exit(1);
                    });
                write_air_table(&air_prog, &vm_trace, &path).unwrap_or_else(|e| {
                    eprintln!("failed to write AIR output: {e}");
                    process::exit(1);
                });
            }

            println!("proof written to {} ({}KB)", out_path, bytes.len() / 1024);
            println!("prove_ms={prove_ms:.3}");
        }
        Command::Verify {
            program,
            proof,
            public,
            claim,
        } => {
            let prog = parse_file(&program).unwrap_or_else(|e| {
                eprintln!("parse error: {e}");
                process::exit(1);
            });

            let symbols = load_symbols(&program);
            let claim = claim.map(|c| {
                resolve_claim(&c, &symbols).unwrap_or_else(|e| {
                    eprintln!("invalid claim: {e}");
                    process::exit(1);
                })
            });

            let proof_bytes = std::fs::read(&proof).unwrap_or_else(|e| {
                eprintln!("failed to read proof: {e}");
                process::exit(1);
            });

            let proof = prover::Proof::from_bytes(&proof_bytes).unwrap_or_else(|e| {
                eprintln!("invalid proof file: {e}");
                process::exit(1);
            });

            let timer = Instant::now();
            let result = match &claim {
                Some(c) => verify_with_claim(&prog, proof, &public, c),
                None => verify_with_inputs(&prog, proof, &public),
            };
            let verify_ms = timer.elapsed().as_secs_f64() * 1000.0;

            match result {
                Ok(()) => {
                    println!("verification succeeded");
                    println!("verify_ms={verify_ms:.3}");
                }
                Err(e) => {
                    eprintln!("verification failed: {e}");
                    process::exit(1);
                }
            }
        }
    }
}
