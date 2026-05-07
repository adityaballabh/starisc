import ast
from .op import Op

BINOP_MAP = {ast.Add: "ADD", ast.Sub: "SUB", ast.Mult: "MUL", ast.Mod: "MOD"}


class Flattener(ast.NodeVisitor):
    def __init__(self):
        self._ops = []
        self._next_temp = 0
        self._consts = {}
        self._assigned_names = set()

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
        return self._emit_value_op(opcode, lhs, rhs)

    def _emit_value_op(self, opcode, left, right):
        t = self._incr_temp()
        self._ops.append(Op(opcode, t, left, right))
        return t

    def _negate_lt(self, left, right):
        lt = self._emit_binary("LT", left, right)
        return self._negate_bool(lt)

    def _negate_bool(self, value):
        one = self._incr_temp()
        self._ops.append(Op("SET", one, "1"))
        t = self._incr_temp()
        self._ops.append(Op("SUB", t, one, value))
        return t

    def _const_int(self, node):
        match node:
            case ast.Constant(value=v) if isinstance(v, int):
                return v
            case ast.Name(id=name):
                return self._consts.get(name)
            case _:
                return None

    def _flatten_statements(self, statements, consts):
        saved_ops = self._ops
        saved_consts = self._consts
        saved_assigned_names = self._assigned_names

        self._ops = []
        self._consts = dict(consts)
        self._assigned_names = set()

        try:
            for stmt in statements:
                self.visit(stmt)
            return self._ops, self._consts, self._assigned_names
        finally:
            self._ops = saved_ops
            self._consts = saved_consts
            self._assigned_names = saved_assigned_names

    def _flatten_condition(self, node):
        match node:
            case ast.UnaryOp(op=ast.Not(), operand=operand):
                return self._negate_bool(self._flatten_condition(operand))

            case ast.BoolOp():
                raise NotImplementedError(
                    f"unsupported condition: {ast.dump(node, include_attributes=False)}"
                )

            case ast.Compare(left=left, ops=[op], comparators=[right]):
                lhs = self._flatten_expr(left)
                rhs = self._flatten_expr(right)

                match op:
                    case ast.Lt():
                        return self._emit_value_op("LT", lhs, rhs)
                    case ast.Gt():
                        return self._emit_value_op("LT", rhs, lhs)
                    case ast.LtE():
                        return self._negate_bool(self._emit_value_op("LT", rhs, lhs))
                    case ast.GtE():
                        return self._negate_bool(self._emit_value_op("LT", lhs, rhs))
                    case ast.Eq():
                        left_lt_right = self._emit_value_op("LT", lhs, rhs)
                        right_lt_left = self._emit_value_op("LT", rhs, lhs)
                        not_equal = self._emit_value_op(
                            "ADD", left_lt_right, right_lt_left
                        )
                        return self._negate_bool(not_equal)
                    case ast.NotEq():
                        left_lt_right = self._emit_value_op("LT", lhs, rhs)
                        right_lt_left = self._emit_value_op("LT", rhs, lhs)
                        return self._emit_value_op("ADD", left_lt_right, right_lt_left)
                    case _:
                        raise NotImplementedError(
                            f"unsupported condition: {ast.dump(node, include_attributes=False)}"
                        )

            case _:
                value = self._flatten_expr(node)
                return self._emit_value_op("LT", "r0", value)

    def _flatten_expr(self, node):
        match node:
            case ast.Name(id=name):
                return name

            case ast.Constant(value=v):
                t = self._incr_temp()
                self._ops.append(Op("SET", t, str(v)))
                return t

            # (base ** exp) % mod -> perform MOD after each MUL
            case ast.BinOp(
                left=ast.BinOp(left=base, op=ast.Pow(), right=exp_node),
                op=ast.Mod(),
                right=mod_node,
            ):
                exp = self._const_int(exp_node)
                if exp is None or exp < 0:
                    raise TypeError(
                        "exponent must be a non-negative integer constant, "
                        f"got {ast.dump(exp_node)}"
                    )
                b = self._flatten_expr(base)
                m = self._flatten_expr(mod_node)
                if exp == 0:
                    t = self._incr_temp()
                    self._ops.append(Op("SET", t, "1"))
                    return t
                res = b
                bits = bin(exp)[2:]
                for bit in bits[1:]:
                    sq = self._incr_temp()
                    self._ops.append(Op("MUL", sq, res, res))
                    sq_mod = self._incr_temp()
                    self._ops.append(Op("MOD", sq_mod, sq, m))
                    if bit == "1":
                        mul = self._incr_temp()
                        self._ops.append(Op("MUL", mul, sq_mod, b))
                        mul_mod = self._incr_temp()
                        self._ops.append(Op("MOD", mul_mod, mul, m))
                        res = mul_mod
                    else:
                        res = sq_mod
                return res

            case ast.BinOp(left=base, op=ast.Pow(), right=exp_node):
                # exponent must be a compile-time constant (otherwise loops are needed)
                exp = self._const_int(exp_node)
                if exp is None or exp < 0:
                    raise TypeError(
                        "exponent must be a non-negative integer constant, "
                        f"got {ast.dump(exp_node)}"
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

            case ast.Call(
                func=ast.Name(id=func_name),
                args=[ast.Constant(value=slot)],
            ) if func_name in ("private", "public"):
                if not isinstance(slot, int) or slot < 0:
                    raise TypeError(f"{func_name} slot must be a non-negative integer")
                opcode = "READ_PRIV" if func_name == "private" else "READ_PUB"
                t = self._incr_temp()
                self._ops.append(Op(opcode, t, str(slot)))
                return t

            case _:
                raise NotImplementedError(f"unsupported expression: {ast.dump(node)}")

    def _assign_to(self, dest, value_node):
        const_value = self._const_int(value_node)
        prev_ops_len = len(self._ops)
        result = self._flatten_expr(value_node)
        if result == dest:
            if const_value is not None:
                self._consts[dest] = const_value
            else:
                self._consts.pop(dest, None)
            return
        if prev_ops_len == len(self._ops):
            # no op was emitted -> SET
            self._ops.append(Op("SET", dest, result))
            if const_value is not None:
                self._consts[dest] = const_value
            else:
                self._consts.pop(dest, None)
            return
        last = self._ops[-1]
        self._ops[-1] = Op(last.opcode, dest, last.src1, last.src2)
        self._next_temp -= 1
        if const_value is not None:
            self._consts[dest] = const_value
        else:
            self._consts.pop(dest, None)

    def visit_Assign(self, node):
        dest = node.targets[0]
        if not isinstance(dest, ast.Name):
            raise TypeError(f"unsupported target: {type(dest).__name__}")

        self._assign_to(dest.id, node.value)
        self._assigned_names.add(dest.id)

    def visit_If(self, node):
        condition = self._flatten_condition(node.test)
        consts_after_condition = dict(self._consts)

        then_ops, _, then_assigned = self._flatten_statements(
            node.body, consts_after_condition
        )
        else_ops, _, else_assigned = self._flatten_statements(
            node.orelse, consts_after_condition
        )

        if else_ops:
            self._ops.append(Op("JZ", condition, str(len(then_ops) + 1)))
            self._ops.extend(then_ops)
            self._ops.append(Op("JZ", "r0", str(len(else_ops))))
            self._ops.extend(else_ops)
        else:
            self._ops.append(Op("JZ", condition, str(len(then_ops))))
            self._ops.extend(then_ops)

        assigned_in_branches = then_assigned | else_assigned
        self._consts = {
            name: value
            for name, value in consts_after_condition.items()
            if name not in assigned_in_branches
        }
        self._assigned_names.update(assigned_in_branches)

    def generic_visit(self, node):
        if isinstance(node, ast.stmt):
            raise NotImplementedError(f"{type(node).__name__} is not supported")
        super().generic_visit(node)

    def visit_Expr(self, node):
        match node.value:
            case ast.Call(func=ast.Name(id="claim"), args=[ast.Name(id=name)]):
                # prevents DCE from eliminating the variable. CLAIM op is not emitted to VM
                self._ops.append(Op("CLAIM", name))
            case _:
                raise NotImplementedError(
                    f"unsupported expression statement at line {node.lineno}"
                )

    def visit_Assert(self, node):
        test = node.test
        if not isinstance(test, ast.Compare) or len(test.ops) != 1:
            raise NotImplementedError(f"expected a single comparison at {node.lineno}")
        if not isinstance(test.ops[0], ast.Eq):
            raise NotImplementedError(f"assert only supports == at {node.lineno}")

        lhs = self._flatten_expr(test.left)
        rhs = self._flatten_expr(test.comparators[0])
        self._ops.append(Op("ASSERT_EQ", lhs, rhs))
