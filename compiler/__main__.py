import sys
from pathlib import Path

from .pipeline import compile_with_symbols


def main():
    if len(sys.argv) != 2:
        print("usage: python -m compiler <input.py>", file=sys.stderr)
        sys.exit(1)
    input_path = Path(sys.argv[1])
    output_path = input_path.with_suffix(".op")
    symbols_path = input_path.with_suffix(".symbols")
    program = input_path.read_text()
    op_text, symbols = compile_with_symbols(program)
    output_path.write_text(op_text + "\n")
    if symbols:
        symbols_path.write_text(
            "\n".join(f"{name} {reg}" for name, reg in sorted(symbols.items())) + "\n"
        )


if __name__ == "__main__":
    main()
