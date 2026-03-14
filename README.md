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

## Pipeline

`.py` → `Compiler` → `.op` → `Parser` → `Interpreter` → `Trace` → `Winterfell Prover`
