import sys
from pathlib import Path

from .pipeline import compile_to_op


def main():
    if len(sys.argv) != 2:
        print("usage: python -m compiler <input.py>", file=sys.stderr)
        sys.exit(1)
    input_path = Path(sys.argv[1])
    output_path = input_path.with_suffix(".op")
    program = input_path.read_text()
    output_path.write_text(compile_to_op(program) + "\n")


if __name__ == "__main__":
    main()
