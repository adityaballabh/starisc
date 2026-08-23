import argparse
from pathlib import Path

from .pipeline import compile_with_symbols

OUT_DIR_FLAG = "--out-dir"
CONST_FLAG = "--const"
OP_EXT = "op"
SYMBOLS_EXT = "symbols"


def parse_const(value: str) -> tuple[str, int]:
    name, raw = value.split("=", 1)
    if not name.isidentifier():
        raise argparse.ArgumentTypeError(f"invalid const name: {name}")
    try:
        return name, int(raw)
    except ValueError as exc:
        raise argparse.ArgumentTypeError(f"invalid const value: {raw}") from exc


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("input")
    parser.add_argument(OUT_DIR_FLAG)
    parser.add_argument(
        CONST_FLAG, dest="consts", action="append", default=[], type=parse_const
    )
    parser.add_argument("--name")
    args = parser.parse_args()

    input_path = Path(args.input)
    out_dir = Path(args.out_dir) if args.out_dir else input_path.parent
    out_dir.mkdir(parents=True, exist_ok=True)

    output_stem = args.name or input_path.stem
    output_path = out_dir / f"{output_stem}.{OP_EXT}"
    symbols_path = out_dir / f"{output_stem}.{SYMBOLS_EXT}"
    program = input_path.read_text()
    op_text, symbols = compile_with_symbols(program, dict(args.consts))
    output_path.write_text(op_text + "\n")
    symbols_text = "\n".join(f"{name} {reg}" for name, reg in sorted(symbols.items()))
    symbols_path.write_text(symbols_text + ("\n" if symbols_text else ""))


if __name__ == "__main__":
    main()
