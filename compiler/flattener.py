import ast
from .op import Op

BINOP_MAP = {ast.Add: "ADD", ast.Sub: "SUB", ast.Mult: "MUL", ast.Mod: "MOD"}


class Flattener(ast.NodeVisitor):
    def __init__(self):
        self._ops = []
        self._next_temp = 0

    def run(self, tree):
        self.visit(tree)
        return self._ops

    def _incr_temp(self):
        temp_name = f"t{self._next_temp}"
        self._next_temp += 1
        return temp_name

    def _emit_binary(self, opcode, left, right):
        lhs = self._flatten_expr(left)
        rhs = self._flatten_expr(right)
        t = self._incr_temp()
        self._ops.append(Op(opcode, t, lhs, rhs))
        return t

    def _negate_lt(self, left, right):
        lt = self._emit_binary("LT", left, right)
        one = self._incr_temp()
        self._ops.append(Op("SET", one, "1"))
        t = self._incr_temp()
        self._ops.append(Op("SUB", t, one, lt))
        return t

    def _flatten_expr(self, node):
        match node:
            case ast.Name(id=name):
                return name

            case ast.Constant(value=v):
                t = self._incr_temp()
                self._ops.append(Op("SET", t, str(v)))
                return t

            case ast.BinOp(left=base, op=ast.Pow(), right=ast.Constant(value=exp)):
                # exponent must be a compile-time constant (otherwise loops are needed)
                if not isinstance(exp, int) or exp < 0:
                    raise TypeError(
                        f"exponent must be a non-negative integer constant, got {exp}"
                    )
                b = self._flatten_expr(base)
                if exp == 0:
                    t = self._incr_temp()
                    self._ops.append(Op("SET", t, "1"))
                    return t
                # binary exponentiation
                bits = bin(exp)[2:]
                res = b
                for bit in bits[1:]:
                    sq = self._incr_temp()
                    self._ops.append(Op("MUL", sq, res, res))
                    if bit == "1":
                        mul = self._incr_temp()
                        self._ops.append(Op("MUL", mul, sq, b))
                        res = mul
                    else:
                        res = sq
                return res

            case ast.BinOp(left=left, op=op, right=right):
                opcode = BINOP_MAP.get(type(op))
                if opcode is None:
                    raise TypeError(f"unsupported operator: {type(op).__name__}")
                return self._emit_binary(opcode, left, right)

            case ast.Compare(left=left, ops=[ast.Lt()], comparators=[right]):
                return self._emit_binary("LT", left, right)

            # lte: a <= b <-> 1 - (b < a)
            case ast.Compare(left=left, ops=[ast.LtE()], comparators=[right]):
                return self._negate_lt(right, left)

            # convert gt to lt
            case ast.Compare(left=left, ops=[ast.Gt()], comparators=[right]):
                return self._emit_binary("LT", right, left)

            case ast.Compare(left=left, ops=[ast.GtE()], comparators=[right]):
                return self._negate_lt(left, right)

            case _:
                raise NotImplementedError(f"unsupported expression: {ast.dump(node)}")

    def _assign_to(self, dest, value_node):
        prev_ops_len = len(self._ops)
        result = self._flatten_expr(value_node)
        if result == dest:
            return
        if prev_ops_len == len(self._ops):
            # no op was emitted -> SET
            self._ops.append(Op("SET", dest, result))
            return
        last = self._ops[-1]
        self._ops[-1] = Op(last.opcode, dest, last.src1, last.src2)
        self._next_temp -= 1

    def visit_Assign(self, node):
        dest = node.targets[0]
        if not isinstance(dest, ast.Name):
            raise TypeError(f"unsupported target: {type(dest).__name__}")

        self._assign_to(dest.id, node.value)

    def generic_visit(self, node):
        if isinstance(node, ast.stmt):
            raise NotImplementedError(f"{type(node).__name__} is not supported")
        super().generic_visit(node)

    def visit_Assert(self, node):
        test = node.test
        if not isinstance(test, ast.Compare) or len(test.ops) != 1:
            raise NotImplementedError(f"expected a single comparison at {node.lineno}")
        if not isinstance(test.ops[0], ast.Eq):
            raise NotImplementedError(f"assert only supports == at {node.lineno}")

        lhs = self._flatten_expr(test.left)
        rhs = self._flatten_expr(test.comparators[0])
        self._ops.append(Op("ASSERT_EQ", lhs, rhs))
