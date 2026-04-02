import sys
from .pipeline import compiler


def main():
    if len(sys.argv) != 2:
        print("usage: python -m compiler <input.py>", file=sys.stderr)
        sys.exit(1)
    file_path = sys.argv[1]
    with open(file_path) as file:
        prog = file.read()
    ops = compiler(prog)
    for op in ops:
        print(op)


if __name__ == "__main__":
    main()
