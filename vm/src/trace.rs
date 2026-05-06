use crate::instruction::Instruction;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::io::Result as IoResult;
use std::iter::once;

fn used_registers(prog: &[Instruction]) -> Vec<u8> {
    let mut used = [false; 16];
    for instr in prog {
        match instr {
            Instruction::Set { dest, .. }
            | Instruction::Add { dest, .. }
            | Instruction::Sub { dest, .. }
            | Instruction::Mul { dest, .. }
            | Instruction::Mod { dest, .. }
            | Instruction::Lt { dest, .. } => used[*dest as usize] = true,
            Instruction::AssertEq { .. } | Instruction::Jz { .. } => {}
        }
    }
    (1..16).filter(|&reg| used[reg as usize]).collect()
}

pub fn write_trace_table(prog: &[Instruction], trace: &Trace, path: &str) -> IoResult<()> {
    let regs = used_registers(prog);

    let col_widths: Vec<usize> = regs
        .iter()
        .map(|&reg| {
            let col_width = format!("r{}", reg).len();
            let max_val_width = trace
                .iter()
                .map(|row| format!("{}", row.registers[reg as usize]).len())
                .max()
                .unwrap_or(1);
            col_width.max(max_val_width)
        })
        .collect();

    let mut out = String::new();

    const PC_WIDTH: usize = 4;
    const INSTR_WIDTH: usize = 18;

    let headers: Vec<String> = regs
        .iter()
        .zip(&col_widths)
        .map(|(&r, w)| format!("{:>width$}", format!("r{}", r), width = w + 2))
        .collect();
    writeln!(
        out,
        "{:>PC_WIDTH$}  {:<INSTR_WIDTH$}  {}",
        "PC",
        "Instruction",
        headers.join("")
    )
    .unwrap();

    let separator_width =
        PC_WIDTH + 2 + INSTR_WIDTH + 2 + col_widths.iter().map(|w| w + 2).sum::<usize>();
    writeln!(out, "{}", "-".repeat(separator_width)).unwrap();

    for (i, instr) in prog.iter().enumerate() {
        let row = &trace[i];
        let reg_vals: Vec<u64> = regs.iter().map(|&r| row.registers[r as usize]).collect();
        let formatted_reg_vals: Vec<String> = reg_vals
            .iter()
            .zip(&col_widths)
            .map(|(v, w)| format!("{:>width$}", v, width = w + 2))
            .collect();
        writeln!(
            out,
            "{:>PC_WIDTH$}  {:<INSTR_WIDTH$}  {}",
            i,
            format!("{}", instr).trim(),
            formatted_reg_vals.join("")
        )
        .unwrap();
    }

    fs::write(path, out)
}

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
) -> IoResult<()> {
    let mut out = String::new();
    let all_rows: Vec<[u64; 16]> = once([0; 16])
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
