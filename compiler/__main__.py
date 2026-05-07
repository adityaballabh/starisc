import argparse
from pathlib import Path

from .pipeline import compile_with_symbols


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("input")
    parser.add_argument("--out-dir")
    args = parser.parse_args()

    input_path = Path(args.input)
    out_dir = Path(args.out_dir) if args.out_dir else input_path.parent
    out_dir.mkdir(parents=True, exist_ok=True)

    output_path = out_dir / f"{input_path.stem}.op"
    symbols_path = out_dir / f"{input_path.stem}.symbols"
    program = input_path.read_text()
    op_text, symbols = compile_with_symbols(program)
    output_path.write_text(op_text + "\n")
    if symbols:
        symbols_path.write_text(
            "\n".join(f"{name} {reg}" for name, reg in sorted(symbols.items())) + "\n"
        )


if __name__ == "__main__":
    main()
