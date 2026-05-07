use crate::trace_builder::build_trace;
use crate::{
    ACTIVE_COL, COND_COL, NUM_RANGE_BITS, NUM_REGISTERS, QUOT_COL, RANGE_BITS_BASE, RES_COL,
    SKIP_COUNTDOWN_COL, SRC1_COL, SRC2_COL, WRAP_BITS_BASE,
};
use std::fmt::Write as FmtWrite;
use std::fs;
use std::io::Result as IoResult;
use vm::{Instruction, Trace};
use winterfell::math::StarkField;

pub fn write_air_table(prog: &[Instruction], vm_trace: &Trace, path: &str) -> IoResult<()> {
    let trace = build_trace(prog, vm_trace);

    let mut rows = Vec::with_capacity(prog.len() + 1);
    rows.push(AirDisplayRow {
        row: "0".to_string(),
        instruction: "INIT".to_string(),
        regs: "-".to_string(),
        src1: "0".to_string(),
        src2: "0".to_string(),
        res: "0".to_string(),
        quot: "0".to_string(),
        active: trace_u64(&trace, ACTIVE_COL, 0).to_string(),
        cond: trace_u64(&trace, COND_COL, 0).to_string(),
        skip: trace_u64(&trace, SKIP_COUNTDOWN_COL, 0).to_string(),
        range: range_value(&trace, 0).to_string(),
        wrap: wrap_value(&trace, 0).to_string(),
    });

    let mut prev_regs = [0u64; NUM_REGISTERS];
    for (pc, instr) in prog.iter().enumerate() {
        let trace_row = pc + 1;
        let curr_regs = std::array::from_fn(|reg| trace_u64(&trace, reg, trace_row));
        let active = trace_u64(&trace, ACTIVE_COL, pc);

        rows.push(AirDisplayRow {
            row: trace_row.to_string(),
            instruction: instr.to_string().trim().to_string(),
            regs: reg_changes(&prev_regs, &curr_regs, instr, active),
            src1: trace_u64(&trace, SRC1_COL, trace_row).to_string(),
            src2: trace_u64(&trace, SRC2_COL, trace_row).to_string(),
            res: trace_u64(&trace, RES_COL, trace_row).to_string(),
            quot: trace_u64(&trace, QUOT_COL, trace_row).to_string(),
            active: active.to_string(),
            cond: trace_u64(&trace, COND_COL, trace_row).to_string(),
            skip: trace_u64(&trace, SKIP_COUNTDOWN_COL, pc).to_string(),
            range: range_value(&trace, trace_row).to_string(),
            wrap: wrap_value(&trace, trace_row).to_string(),
        });

        prev_regs = curr_regs;
    }

    let widths = Widths::from_rows(&rows);
    let mut out = String::new();
    writeln!(
        out,
        "{:>row_w$}  {:<instr_w$}  | {:<regs_w$} | {:>src1_w$}  {:>src2_w$}  {:>res_w$}  {:>quot_w$}  {:>active_w$}  {:>cond_w$}  {:>skip_w$}  {:>range_w$}  {:>wrap_w$}",
        "Row",
        "instruction",
        "regs",
        "src1",
        "src2",
        "res",
        "quot",
        "active",
        "cond",
        "skip",
        "range",
        "wrap",
        row_w = widths.row,
        instr_w = widths.instruction,
        regs_w = widths.regs,
        src1_w = widths.src1,
        src2_w = widths.src2,
        res_w = widths.res,
        quot_w = widths.quot,
        active_w = widths.active,
        cond_w = widths.cond,
        skip_w = widths.skip,
        range_w = widths.range,
        wrap_w = widths.wrap,
    )
    .unwrap();

    let separator_width = widths.total_table_width();
    writeln!(out, "{}", "-".repeat(separator_width)).unwrap();

    for row in rows {
        writeln!(
            out,
            "{:>row_w$}  {:<instr_w$}  | {:<regs_w$} | {:>src1_w$}  {:>src2_w$}  {:>res_w$}  {:>quot_w$}  {:>active_w$}  {:>cond_w$}  {:>skip_w$}  {:>range_w$}  {:>wrap_w$}",
            row.row,
            row.instruction,
            row.regs,
            row.src1,
            row.src2,
            row.res,
            row.quot,
            row.active,
            row.cond,
            row.skip,
            row.range,
            row.wrap,
            row_w = widths.row,
            instr_w = widths.instruction,
            regs_w = widths.regs,
            src1_w = widths.src1,
            src2_w = widths.src2,
            res_w = widths.res,
            quot_w = widths.quot,
            active_w = widths.active,
            cond_w = widths.cond,
            skip_w = widths.skip,
            range_w = widths.range,
            wrap_w = widths.wrap,
        )
        .unwrap();
    }

    fs::write(path, out)
}

fn trace_u64(trace: &winterfell::TraceTable<crate::Felt>, col: usize, row: usize) -> u64 {
    trace.get(col, row).as_int() as u64
}

fn reconstruct_bits(
    trace: &winterfell::TraceTable<crate::Felt>,
    base_col: usize,
    row: usize,
) -> u64 {
    let mut value = 0u64;
    for bit in 0..NUM_RANGE_BITS {
        value |= trace_u64(trace, base_col + bit, row) << bit;
    }
    value
}

fn range_value(trace: &winterfell::TraceTable<crate::Felt>, row: usize) -> u64 {
    reconstruct_bits(trace, RANGE_BITS_BASE, row)
}

fn wrap_value(trace: &winterfell::TraceTable<crate::Felt>, row: usize) -> u64 {
    reconstruct_bits(trace, WRAP_BITS_BASE, row)
}

fn reg_changes(
    prev: &[u64; NUM_REGISTERS],
    curr: &[u64; NUM_REGISTERS],
    instr: &Instruction,
    active: u64,
) -> String {
    if active == 0 {
        return "-".to_string();
    }

    let changes: Vec<String> = (0..NUM_REGISTERS)
        .filter(|&reg| prev[reg] != curr[reg])
        .map(|reg| format!("r{}={}", reg, curr[reg]))
        .collect();

    if changes.is_empty() {
        if matches!(instr, Instruction::AssertEq { .. }) {
            "passed".to_string()
        } else {
            "-".to_string()
        }
    } else {
        changes.join("  ")
    }
}

struct AirDisplayRow {
    row: String,
    instruction: String,
    regs: String,
    src1: String,
    src2: String,
    res: String,
    quot: String,
    active: String,
    cond: String,
    skip: String,
    range: String,
    wrap: String,
}

struct Widths {
    row: usize,
    instruction: usize,
    regs: usize,
    src1: usize,
    src2: usize,
    res: usize,
    quot: usize,
    active: usize,
    cond: usize,
    skip: usize,
    range: usize,
    wrap: usize,
}

impl Widths {
    fn from_rows(rows: &[AirDisplayRow]) -> Self {
        Self {
            row: width(rows.iter().map(|row| row.row.as_str()), "Row"),
            instruction: width(
                rows.iter().map(|row| row.instruction.as_str()),
                "instruction",
            ),
            regs: width(rows.iter().map(|row| row.regs.as_str()), "regs"),
            src1: width(rows.iter().map(|row| row.src1.as_str()), "src1"),
            src2: width(rows.iter().map(|row| row.src2.as_str()), "src2"),
            res: width(rows.iter().map(|row| row.res.as_str()), "res"),
            quot: width(rows.iter().map(|row| row.quot.as_str()), "quot"),
            active: width(rows.iter().map(|row| row.active.as_str()), "active"),
            cond: width(rows.iter().map(|row| row.cond.as_str()), "cond"),
            skip: width(rows.iter().map(|row| row.skip.as_str()), "skip"),
            range: width(rows.iter().map(|row| row.range.as_str()), "range"),
            wrap: width(rows.iter().map(|row| row.wrap.as_str()), "wrap"),
        }
    }

    fn total_table_width(&self) -> usize {
        self.row
            + 2
            + self.instruction
            + 3
            + self.regs
            + 3
            + self.src1
            + 2
            + self.src2
            + 2
            + self.res
            + 2
            + self.quot
            + 2
            + self.active
            + 2
            + self.cond
            + 2
            + self.skip
            + 2
            + self.range
            + 2
            + self.wrap
    }
}

fn width<'a>(values: impl Iterator<Item = &'a str>, header: &str) -> usize {
    values.map(str::len).max().unwrap_or(0).max(header.len())
}
