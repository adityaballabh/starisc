# StaRISC

StaRISC (STARK RISC) is a minimal zkVM with a restricted 7-opcode ISA built on top of the Winterfell STARK prover.

## Instruction Set

| Opcode | Syntax | Semantics |
|---|---|---|
| `SET` | `SET r val` | `r = val` |
| `ADD` | `ADD dest src1 src2` | `dest = src1 + src2` |
| `SUB` | `SUB dest src1 src2` | `dest = src1 - src2` |
| `MUL` | `MUL dest src1 src2` | `dest = src1 * src2` |
| `MOD` | `MOD dest src1 src2` | `dest = src1 % src2` |
| `ASSERT_EQ` | `ASSERT_EQ r1 r2` | halt if `r1 != r2` |
| `LT` | `LT dest src1 src2` | `dest = (src1 < src2) as u64` |

16 registers `r0`-`r15`. `r0` is mapped to zero (avoids MOV): reads always return zero, writes are a parse error. All arithmetic is wrapping `u64`.

## Pipeline

`.py` → `Compiler` → `.op` → `Parser` → `Interpreter` → `Trace` → `Winterfell Prover`

## Progress

-  Parser — reads `.op` files into `Vec<Instruction>`
-  Interpreter — executes `Vec<Instruction>`, returns `(Trace, final_registers)`. Trace contains the snapshot of all registers after each instruction.


## Usage

```
cargo run -p vm
```

Executes [`examples/sample.op`](examples/sample.op) and writes a trace to [`logs/trace.log`](logs/trace.log)
