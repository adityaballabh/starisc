# StaRISC

StaRISC (STARK RISC) is a minimal zkVM with a restricted 10-opcode ISA built on top of the Winterfell STARK prover.

## Instruction Set

| Opcode      | Syntax               | Semantics                     |
| ----------- | -------------------- | ----------------------------- |
| `SET`       | `SET r val`          | `r = val`                     |
| `ADD`       | `ADD dest src1 src2` | `dest = src1 + src2`          |
| `SUB`       | `SUB dest src1 src2` | `dest = src1 - src2`          |
| `MUL`       | `MUL dest src1 src2` | `dest = src1 * src2`          |
| `MOD`       | `MOD dest src1 src2` | `dest = src1 % src2`          |
| `ASSERT_EQ` | `ASSERT_EQ r1 r2`    | assert `r1 == r2`            |
| `LT`        | `LT dest src1 src2`  | `dest = (src1 < src2) as u64` |
| `JZ`        | `JZ r offset`     | if `r == 0`, skip `offset` |
| `READ_PRIV` | `READ_PRIV dest slot` | `dest = private_inputs[slot]` |
| `READ_PUB`  | `READ_PUB dest slot`  | `dest = public_inputs[slot]`  |

16 registers `r0`-`r15`. `r0` is mapped to zero (avoids MOV): reads always return zero, writes are a parse error. All arithmetic is wrapping `u64`.

Values can be supplied at proving time through `READ_PRIV` and `READ_PUB`. Private inputs are witness-only. Public inputs and claims are verifier-bound.

## Pipeline

.py → `Compiler Frontend` → IR → `Compiler Backend` → .op → `Parser` → `Interpreter` → Trace → `Winterfell Prover` → Proof

## Components

### Compiler

- Compiler frontend

  Flattens and converts .py files into intermediate representation (IR)

- Compiler backend
  Converts IR into `.op` files

### VM

- Parser

  Reads `.op` files into `Vec<Instruction>`

- Interpreter

  Executes `Vec<Instruction>`, returns `(Trace, final_registers)`. Trace contains the snapshot of all registers after each instruction

### Winterfell Prover

- Prover

  Generates a STARK proof for an execution trace using supplied public and private inputs. Proofs with private inputs require a public claim, which StaRISC enforces by automatically inserting an assertion.


- Verifier

  Checks the STARK proof against the AIR constraints using the public program, public inputs, and claim. It does not receive private inputs.

## Usage

StaRISC accepts Python-like programs and compiles them to `.op`. See [starisc-bench/programs](starisc-bench/programs/) for examples.

Compile with:

```bash
python3 -m compiler program.py
```

Prove with private inputs, public inputs, and a public claim:
```bash
cargo run --release -p starisc-cli -- prove program.op --private 37 --public 5 --claim z=42
```

Verify with public inputs and the same claim:
```bash
cargo run --release -p starisc-cli -- verify program.op --proof program.op.proof --public 5 --claim z=42
```

### Benchmarks

Programs include `rsa_enc`, `rsa_dec`, `fib_8`, and `fib_16`.

| zkVM |  Command | Results |
| ------ | ----------------- | ----------------- |
| StaRISC | `cargo run --release -p starisc-bench` | [starisc-bench/results](starisc-bench/results/) |
| RISC Zero | `cd risczero-bench && cargo run --release` | [risczero-bench/results](risczero-bench/results/) |
| SP1 | `cd sp1-bench && cargo run --release -p sp1-bench-script` | [sp1-bench/results](sp1-bench/results/) |
