import ast
from .op import Op

BINOP_MAP = {ast.Add: "ADD", ast.Sub: "SUB", ast.Mult: "MUL", ast.Mod: "MOD"}


class Walker(ast.NodeVisitor):
    def __init__(self):
        self._ops = []

    def run(self, tree):
        self.visit(tree)
        return self._ops

    def visit_Assign(self, node):
        dest = node.targets[0]
        if not isinstance(dest, ast.Name):
            raise TypeError(f"unsupported target: {type(dest).__name__}")

        match node.value:
            case ast.Constant(value=v):
                self._ops.append(Op("SET", dest.id, str(v)))

            case ast.BinOp(left=ast.Name(id=l), op=op, right=ast.Name(id=r)):
                opcode = BINOP_MAP.get(type(op))
                if opcode is None:
                    raise TypeError(f"unsupported operator: {type(op).__name__}")
                self._ops.append(Op(opcode, dest.id, l, r))

            case ast.Compare(
                left=ast.Name(id=l), ops=[ast.Lt()], comparators=[ast.Name(id=r)]
            ):
                self._ops.append(Op("LT", dest.id, l, r))

            case _:
                raise NotImplementedError(f"complex expression at {node.lineno}")

    def visit_Assert(self, node: ast.Assert):
        test = node.test
        if not isinstance(test, ast.Compare) or len(test.ops) != 1:
            raise TypeError(f"unsupported assertion at {node.lineno}")

        match (test.left, test.ops[0], test.comparators[0]):
            case (ast.Name(id=l), ast.Eq(), ast.Name(id=r)):
                self._ops.append(Op("ASSERT_EQ", l, r))

            case _:
                raise NotImplementedError(f"complex assertion at {node.lineno}")
