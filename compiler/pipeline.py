import ast
from .op import Op
from .walker import Walker


def compiler(source: str) -> list[Op]:
    tree = ast.parse(source)
    return Walker().run(tree)
