use crate::instruction::Instruction;
use std::fmt;
use std::fs;

#[derive(Debug, PartialEq)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

fn parse_register(token: &str, line: usize) -> Result<u8, ParseError> {
    let bad_reg = || ParseError {
        line,
        message: format!("expected register value: r[0-15], got {:?}", token),
    };
    let s = token.strip_prefix('r').ok_or_else(bad_reg)?;
    let idx: u8 = s.parse().map_err(|_| bad_reg())?;
    if idx > 15 {
        return Err(ParseError {
            line,
            message: format!("invalid register value: {:?} (max=15)", token),
        });
    }
    Ok(idx)
}

fn expect_argc(opcode: &str, expected: usize, got: usize, line: usize) -> Result<(), ParseError> {
    if got != expected {
        return Err(ParseError {
            line,
            message: format!("{} expected {} args, got {}", opcode, expected, got),
        });
    }
    Ok(())
}

fn parse_val(token: &str, line: usize) -> Result<u64, ParseError> {
    token.parse().map_err(|_| ParseError {
        line,
        message: format!("expected u64 literal, got {:?}", token),
    })
}

fn parse_arith_args(tokens: &[&str], line: usize) -> Result<(u8, u8, u8), ParseError> {
    Ok((
        parse_dest_register(tokens[1], line)?,
        parse_register(tokens[2], line)?,
        parse_register(tokens[3], line)?,
    ))
}

fn parse_dest_register(token: &str, line: usize) -> Result<u8, ParseError> {
    let idx = parse_register(token, line)?;
    if idx == 0 {
        return Err(ParseError {
            line,
            message: "cannot write to r0".to_string(),
        });
    }
    Ok(idx)
}

pub fn parse_str(input: &str) -> Result<Vec<Instruction>, ParseError> {
    let mut instructions = Vec::new();

    for (i, raw_line) in input.lines().enumerate() {
        let line_num = i + 1;
        let line = raw_line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let tokens: Vec<&str> = line.split_whitespace().collect();
        let opcode = tokens[0];
        let argc = tokens.len() - 1;
        let instr = match opcode {
            "SET" => {
                expect_argc("SET", 2, argc, line_num)?;
                let dest = parse_dest_register(tokens[1], line_num)?;
                let val = parse_val(tokens[2], line_num)?;
                Instruction::Set { dest, val }
            }
            "ASSERT_EQ" => {
                expect_argc("ASSERT_EQ", 2, argc, line_num)?;
                Instruction::AssertEq {
                    r1: parse_register(tokens[1], line_num)?,
                    r2: parse_register(tokens[2], line_num)?,
                }
            }
            "ADD" => {
                expect_argc("ADD", 3, argc, line_num)?;
                let (dest, src1, src2) = parse_arith_args(&tokens, line_num)?;
                Instruction::Add { dest, src1, src2 }
            }
            "SUB" => {
                expect_argc("SUB", 3, argc, line_num)?;
                let (dest, src1, src2) = parse_arith_args(&tokens, line_num)?;
                Instruction::Sub { dest, src1, src2 }
            }
            "MUL" => {
                expect_argc("MUL", 3, argc, line_num)?;
                let (dest, src1, src2) = parse_arith_args(&tokens, line_num)?;
                Instruction::Mul { dest, src1, src2 }
            }
            "MOD" => {
                expect_argc("MOD", 3, argc, line_num)?;
                let (dest, src1, src2) = parse_arith_args(&tokens, line_num)?;
                Instruction::Mod { dest, src1, src2 }
            }
            "LT" => {
                expect_argc("LT", 3, argc, line_num)?;
                let (dest, src1, src2) = parse_arith_args(&tokens, line_num)?;
                Instruction::Lt { dest, src1, src2 }
            }
            other => {
                return Err(ParseError {
                    line: line_num,
                    message: format!("unknown opcode {:?}", other),
                });
            }
        };
        instructions.push(instr);
    }

    Ok(instructions)
}

pub fn parse_file(path: &str) -> Result<Vec<Instruction>, ParseError> {
    let contents = fs::read_to_string(path).map_err(|e| ParseError {
        line: 0,
        message: format!("error while reading file: {}", e),
    })?;
    parse_str(&contents)
}
