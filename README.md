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

.py → `Compiler Frontend` → IR → `Compiler Backend` → .op → `Parser` → `Interpreter` → Trace → `Winterfell Prover` → Proof

## Progress
### Compiler
-  Compiler frontend

    Flattens and converts .py files into intermediate representation (IR)
-  Compiler backend
    
    Converts IR into `.op` files
### VM
-  Parser

    Reads `.op` files into `Vec<Instruction>`
-  Interpreter

    Executes `Vec<Instruction>`, returns `(Trace, final_registers)`. Trace contains the snapshot of all registers after each instruction
### Winterfell Prover
-  Prover

    Proves the execution trace satisfies the AIR constraints and generates a STARK proof
-  Verifier

    Checks the STARK proof against the AIR constraints

### Benchmarking
- Benchmarks StaRISC against RiscZero for simple programs

    [StaRISC Results](starisc-bench/results/)

    [RISC Zero Results](risczero-bench/results/)

## Usage

  Run StaRISC benchmarks
  ```
  cargo run --release -p starisc-bench
  ```

  Run RiscZero benchmarks
  ```
  cd risczero-bench && cargo run --release
  ```