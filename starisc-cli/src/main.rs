use clap::{Parser, Subcommand};
use prover::prover::{
    prove_with_claim, prove_with_inputs, verify_with_claim, verify_with_inputs, Claim,
};
use std::process;
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
        #[arg(long, value_parser = parse_claim)]
        claim: Option<Claim>,
        #[arg(short, long)]
        output: Option<String>,
    },
    Verify {
        program: String,
        #[arg(long)]
        proof: String,
        #[arg(long, value_delimiter = ',')]
        public: Vec<u64>,
        #[arg(long, value_parser = parse_claim)]
        claim: Option<Claim>,
    },
}

fn parse_claim(s: &str) -> Result<Claim, String> {
    let s = s.trim();
    if !s.starts_with('r') {
        return Err("claim must be of the form r<N>=<value>".into());
    }
    let rest = &s[1..];
    let (reg_str, val_str) = rest
        .split_once('=')
        .ok_or("claim must be of the form r<N>=<value>")?;
    let register: u8 = reg_str.parse().map_err(|_| "invalid register number")?;
    if register == 0 || register > 15 {
        return Err("register must be r1-r15".into());
    }
    let expected: u64 = val_str.parse().map_err(|_| "invalid claim value")?;
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
        } => {
            let prog = parse_file(&program).unwrap_or_else(|e| {
                eprintln!("parse error: {e}");
                process::exit(1);
            });

            let proof = match &claim {
                Some(c) => prove_with_claim(&prog, &private, &public, c),
                None => prove_with_inputs(&prog, &private, &public),
            };

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

            println!("proof written to {} ({}KB)", out_path, bytes.len() / 1024);
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

            let proof_bytes = std::fs::read(&proof).unwrap_or_else(|e| {
                eprintln!("failed to read proof: {e}");
                process::exit(1);
            });

            let proof = prover::Proof::from_bytes(&proof_bytes).unwrap_or_else(|e| {
                eprintln!("invalid proof file: {e}");
                process::exit(1);
            });

            let result = match &claim {
                Some(c) => verify_with_claim(&prog, proof, &public, c),
                None => verify_with_inputs(&prog, proof, &public),
            };

            match result {
                Ok(()) => println!("verification succeeded"),
                Err(e) => {
                    eprintln!("verification failed: {e}");
                    process::exit(1);
                }
            }
        }
    }
}
