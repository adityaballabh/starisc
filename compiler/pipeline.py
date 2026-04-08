import ast

from .backend import Backend
from .flattener import Flattener
from .op import Op


def compile_first_stage(source: str) -> list[Op]:
    tree = ast.parse(source)
    return Flattener().run(tree)


def compile_second_stage(flattened_ops: list[Op]) -> str:
    return Backend().run(flattened_ops)


def compile_to_op(source: str) -> str:
    return compile_second_stage(compile_first_stage(source))
