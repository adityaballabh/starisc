import ast
from .op import Op
from .flattener import Flattener


def compiler(source: str) -> list[Op]:
    tree = ast.parse(source)
    return Flattener().run(tree)
