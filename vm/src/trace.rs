use crate::instruction::Instruction;
use std::fmt::Write as FmtWrite;
use std::fs;

// dumping trace by default for debugging. change to false before benchmarking
const DUMP_TRACE: bool = true;

#[derive(Debug, Clone, PartialEq)]
pub struct TraceRow {
    pub registers: [u64; 16],
}

pub type Trace = Vec<TraceRow>;

pub fn dump_trace(
    prog: &[Instruction],
    trace: &Trace,
    final_regs: &[u64; 16],
    path: &str,
) -> std::io::Result<()> {
    if !DUMP_TRACE {
        return Ok(());
    }
    let mut out = String::new();
    let all_rows: Vec<[u64; 16]> = std::iter::once([0; 16])
        .chain(trace.iter().map(|r| r.registers))
        .collect();

    for (i, (instr, window)) in prog.iter().zip(all_rows.windows(2)).enumerate() {
        let (prev, curr) = (&window[0], &window[1]);

        let modified: Vec<String> = (0..16)
            .filter(|&r| curr[r] != prev[r])
            .map(|r| format!("r{}={}", r, curr[r]))
            .collect();
        let modified_str = if modified.is_empty() {
            if matches!(instr, Instruction::AssertEq { .. }) {
                "passed".to_string()
            } else {
                "no-op".to_string()
            }
        } else {
            modified.join("  ")
        };
        writeln!(
            out,
            "{:>4}  {:<30}  | {}",
            i,
            format!("{}", instr),
            modified_str
        )
        .unwrap();
    }
    let finals: Vec<String> = (0..16)
        .filter(|&r| final_regs[r] != 0)
        .map(|r| format!("r{}={}", r, final_regs[r]))
        .collect();
    writeln!(
        out,
        "\n{:>4}  {:<30}  | {}",
        "-",
        "FINAL",
        finals.join("  ")
    )
    .unwrap();

    fs::write(path, out)
}
