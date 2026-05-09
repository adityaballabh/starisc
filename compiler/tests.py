import unittest
from pathlib import Path

from .backend import (
    allocate_registers,
    compute_liveness,
    emit_ops,
    op_defs,
    op_uses,
    optimize_ops,
)
from .op import Op
from .pipeline import compile_first_stage, compile_to_op


def read(name):
    return (Path(__file__).parent / "examples" / name).read_text()


class TestSet(unittest.TestCase):
    def test_copy(self):
        self.assertEqual(
            compile_first_stage("a = 88\nb = a"),
            [
                Op("SET", "a", "88"),
                Op("SET", "b", "a"),
            ],
        )

    def test_self_assign(self):
        self.assertEqual(compile_first_stage("c = 23\nc = c"), [Op("SET", "c", "23")])


class TestArithmetic(unittest.TestCase):
    def test_add(self):
        self.assertEqual(
            compile_first_stage("p = 45\nq = 22\nr = p + q"),
            [
                Op("SET", "p", "45"),
                Op("SET", "q", "22"),
                Op("ADD", "r", "p", "q"),
            ],
        )

    def test_sub(self):
        self.assertEqual(
            compile_first_stage("c = 56\nd = 45\na = d - c"),
            [
                Op("SET", "c", "56"),
                Op("SET", "d", "45"),
                Op("SUB", "a", "d", "c"),
            ],
        )

    def test_mul(self):
        self.assertEqual(
            compile_first_stage("x = 23\ny= 7\nz = x * y"),
            [
                Op("SET", "x", "23"),
                Op("SET", "y", "7"),
                Op("MUL", "z", "x", "y"),
            ],
        )

    def test_mod(self):
        self.assertEqual(
            compile_first_stage("n = 38\nm = 7\nr = n % m"),
            [
                Op("SET", "n", "38"),
                Op("SET", "m", "7"),
                Op("MOD", "r", "n", "m"),
            ],
        )


class TestComparisons(unittest.TestCase):
    def test_lt(self):
        self.assertEqual(
            compile_first_stage("r = 24\ns = 91\nres = r < s"),
            [
                Op("SET", "r", "24"),
                Op("SET", "s", "91"),
                Op("LT", "res", "r", "s"),
            ],
        )

    def test_gt(self):
        self.assertEqual(
            compile_first_stage("r = 45\ns = 16\nres = r > s"),
            [
                Op("SET", "r", "45"),
                Op("SET", "s", "16"),
                Op("LT", "res", "s", "r"),
            ],
        )

    def test_gte(self):
        self.assertEqual(
            compile_first_stage("a = 23\nb = 52\nc = a >= b"),
            [
                Op("SET", "a", "23"),
                Op("SET", "b", "52"),
                Op("LT", "t0", "a", "b"),
                Op("SET", "t1", "1"),
                Op("SUB", "c", "t1", "t0"),
            ],
        )

    def test_lte(self):
        self.assertEqual(
            compile_first_stage("p = 19\nq = 21\nr = p <= q"),
            [
                Op("SET", "p", "19"),
                Op("SET", "q", "21"),
                Op("LT", "t0", "q", "p"),
                Op("SET", "t1", "1"),
                Op("SUB", "r", "t1", "t0"),
            ],
        )


class TestPow(unittest.TestCase):
    def test_pow_zero(self):
        self.assertEqual(
            compile_first_stage("g = 37\nres = g ** 0"),
            [
                Op("SET", "g", "37"),
                Op("SET", "res", "1"),
            ],
        )

    def test_pow_one(self):
        self.assertEqual(
            compile_first_stage("g = 52\nres = g ** 1"),
            [
                Op("SET", "g", "52"),
                Op("SET", "res", "g"),
            ],
        )

    def test_pow_five(self):
        self.assertEqual(
            compile_first_stage("a = 7\nb = a ** 5"),
            [
                Op("SET", "a", "7"),
                Op("MUL", "t0", "a", "a"),
                Op("MUL", "t1", "t0", "t0"),
                Op("MUL", "b", "t1", "a"),
            ],
        )

    def test_pow_named_const(self):
        self.assertEqual(
            compile_first_stage("e = 5\na = 7\nb = a ** e"),
            [
                Op("SET", "e", "5"),
                Op("SET", "a", "7"),
                Op("MUL", "t0", "a", "a"),
                Op("MUL", "t1", "t0", "t0"),
                Op("MUL", "b", "t1", "a"),
            ],
        )

    def test_pow_neg_raises(self):
        with self.assertRaises(TypeError):
            compile_first_stage("c = 11\ne = c ** -1")

    def test_pow_const_expr(self):
        ops = compile_first_stage("p = 2\nq = p + 15\nres = p ** q")
        muls = [op for op in ops if op.opcode == "MUL"]
        self.assertTrue(len(muls) > 0)

    def test_pow_reassigned_const(self):
        ops = compile_first_stage("e = 5\ne = e + 1\na = 7\nres = a ** e")
        muls = [op for op in ops if op.opcode == "MUL"]
        self.assertTrue(len(muls) > 0)

    def test_pow_runtime_var_raises(self):
        with self.assertRaises(TypeError):
            compile_first_stage(
                "a = private(0)\nb = private(1)\nres = a ** b\nclaim(res)"
            )


class TestModPow(unittest.TestCase):
    def test_mod_pow_zero(self):
        self.assertEqual(
            compile_first_stage("a = 7\nm = 95\nres = (a ** 0) % m"),
            [
                Op("SET", "a", "7"),
                Op("SET", "m", "95"),
                Op("SET", "res", "1"),
            ],
        )

    def test_mod_pow_five(self):
        self.assertEqual(
            compile_first_stage("a = 452319\nm = 83\nres = (a ** 5) % m"),
            [
                Op("SET", "a", "452319"),
                Op("SET", "m", "83"),
                Op("MUL", "t0", "a", "a"),
                Op("MOD", "t1", "t0", "m"),
                Op("MUL", "t2", "t1", "t1"),
                Op("MOD", "t3", "t2", "m"),
                Op("MUL", "t4", "t3", "a"),
                Op("MOD", "res", "t4", "m"),
            ],
        )

    def test_mod_pow_named_const(self):
        self.assertEqual(
            compile_first_stage("e = 5\na = 234871\nm = 59\nres = (a ** e) % m"),
            [
                Op("SET", "e", "5"),
                Op("SET", "a", "234871"),
                Op("SET", "m", "59"),
                Op("MUL", "t0", "a", "a"),
                Op("MOD", "t1", "t0", "m"),
                Op("MUL", "t2", "t1", "t1"),
                Op("MOD", "t3", "t2", "m"),
                Op("MUL", "t4", "t3", "a"),
                Op("MOD", "res", "t4", "m"),
            ],
        )

    def test_mod_pow_correctness(self):
        result = compile_to_op(
            "m = 4294967296\ns = 2301\ne = (s ** 41) % m\nassert e == 1614534813"
        )
        self.assertIn("MUL", result)
        self.assertIn("MOD", result)
        self.assertIn("ASSERT_EQ", result)


class TestAssert(unittest.TestCase):
    def test_assert_vars(self):
        self.assertEqual(
            compile_first_stage("i = 43\nj = 43\nassert i == j"),
            [
                Op("SET", "i", "43"),
                Op("SET", "j", "43"),
                Op("ASSERT_EQ", "i", "j"),
            ],
        )

    def test_assert_const(self):
        self.assertEqual(
            compile_first_stage("v = 77\nassert v == 77"),
            [
                Op("SET", "v", "77"),
                Op("SET", "t0", "77"),
                Op("ASSERT_EQ", "v", "t0"),
            ],
        )

    def test_assert_nested(self):
        self.assertEqual(
            compile_first_stage("a = 13\nb = 4\nd = 52\nassert a * b == d"),
            [
                Op("SET", "a", "13"),
                Op("SET", "b", "4"),
                Op("SET", "d", "52"),
                Op("MUL", "t0", "a", "b"),
                Op("ASSERT_EQ", "t0", "d"),
            ],
        )

    def test_assert_non_compare_raises(self):
        with self.assertRaises(NotImplementedError):
            compile_first_stage("assert False")


class TestIfLowering(unittest.TestCase):
    def test_if_without_else(self):
        self.assertEqual(
            compile_first_stage("x = 5\nif x:\n    y = 1"),
            [
                Op("SET", "x", "5"),
                Op("LT", "t0", "r0", "x"),
                Op("JZ", "t0", "1"),
                Op("SET", "y", "1"),
            ],
        )

    def test_if_else(self):
        self.assertEqual(
            compile_first_stage("x = 3\nif x:\n    y = 1\nelse:\n    y = 2"),
            [
                Op("SET", "x", "3"),
                Op("LT", "t0", "r0", "x"),
                Op("JZ", "t0", "2"),
                Op("SET", "y", "1"),
                Op("JZ", "r0", "1"),
                Op("SET", "y", "2"),
            ],
        )

    def test_if_eq_condition(self):
        self.assertEqual(
            compile_first_stage("x = 4\ny = 4\nif x == y:\n    z = 9"),
            [
                Op("SET", "x", "4"),
                Op("SET", "y", "4"),
                Op("LT", "t0", "x", "y"),
                Op("LT", "t1", "y", "x"),
                Op("ADD", "t2", "t0", "t1"),
                Op("SET", "t3", "1"),
                Op("SUB", "t4", "t3", "t2"),
                Op("JZ", "t4", "1"),
                Op("SET", "z", "9"),
            ],
        )

    def test_if_neq_condition(self):
        self.assertEqual(
            compile_first_stage("x = 4\ny = 5\nif x != y:\n    z = 9"),
            [
                Op("SET", "x", "4"),
                Op("SET", "y", "5"),
                Op("LT", "t0", "x", "y"),
                Op("LT", "t1", "y", "x"),
                Op("ADD", "t2", "t0", "t1"),
                Op("JZ", "t2", "1"),
                Op("SET", "z", "9"),
            ],
        )

    def test_if_not_truthiness(self):
        self.assertEqual(
            compile_first_stage("x = 5\nif not x:\n    y = 1"),
            [
                Op("SET", "x", "5"),
                Op("LT", "t0", "r0", "x"),
                Op("SET", "t1", "1"),
                Op("SUB", "t2", "t1", "t0"),
                Op("JZ", "t2", "1"),
                Op("SET", "y", "1"),
            ],
        )

    def test_if_not_lt_condition(self):
        self.assertEqual(
            compile_first_stage("x = 2\ny = 3\nif not (x < y):\n    z = 1"),
            [
                Op("SET", "x", "2"),
                Op("SET", "y", "3"),
                Op("LT", "t0", "x", "y"),
                Op("SET", "t1", "1"),
                Op("SUB", "t2", "t1", "t0"),
                Op("JZ", "t2", "1"),
                Op("SET", "z", "1"),
            ],
        )

    def test_nested_if_offsets(self):
        self.assertEqual(
            compile_first_stage(
                "a = 1\nb = 2\nif a:\n    if b:\n        x = 3\n    else:\n        x = 4\nelse:\n    x = 5"
            ),
            [
                Op("SET", "a", "1"),
                Op("SET", "b", "2"),
                Op("LT", "t0", "r0", "a"),
                Op("JZ", "t0", "6"),
                Op("LT", "t1", "r0", "b"),
                Op("JZ", "t1", "2"),
                Op("SET", "x", "3"),
                Op("JZ", "r0", "1"),
                Op("SET", "x", "4"),
                Op("JZ", "r0", "1"),
                Op("SET", "x", "5"),
            ],
        )

    def test_branch_assignment_drops_constant_fact(self):
        source = "e = 5\na = 7\nif a:\n    e = 3\nres = a ** e"
        with self.assertRaises(TypeError):
            compile_first_stage(source)


class TestNested(unittest.TestCase):
    def test_nested_deep(self):
        self.assertEqual(
            compile_first_stage(read("nested_deep.py")),
            [
                Op("SET", "a", "5"),
                Op("SET", "b", "3"),
                Op("SET", "c", "7"),
                Op("SET", "p", "15"),
                Op("SET", "q", "4"),
                Op("SET", "r", "6"),
                Op("SET", "s", "2"),
                Op("SUB", "t0", "b", "c"),
                Op("MUL", "t1", "a", "t0"),
                Op("MOD", "t2", "p", "q"),
                Op("ADD", "t3", "r", "s"),
                Op("MUL", "t4", "t2", "t3"),
                Op("MOD", "res", "t1", "t4"),
            ],
        )

    def test_nested_assert(self):
        self.assertEqual(
            compile_first_stage(read("nested_assert.py")),
            [
                Op("SET", "x", "8"),
                Op("SET", "y", "13"),
                Op("SET", "z", "85"),
                Op("SET", "w", "19"),
                Op("MUL", "t0", "x", "y"),
                Op("ADD", "t1", "w", "z"),
                Op("ASSERT_EQ", "t0", "t1"),
            ],
        )

    def test_nested_precedence(self):
        self.assertEqual(
            compile_first_stage(read("nested_precedence.py")),
            [
                Op("SET", "p", "13"),
                Op("SET", "q", "6"),
                Op("SET", "a", "53"),
                Op("SET", "b", "25"),
                Op("MUL", "t0", "p", "q"),
                Op("MOD", "t1", "a", "b"),
                Op("ADD", "t2", "t0", "t1"),
                Op("SET", "t3", "8"),
                Op("ADD", "res", "t2", "t3"),
            ],
        )


class TestUnsupported(unittest.TestCase):
    def test_unsupported_operator(self):
        with self.assertRaises(TypeError):
            compile_first_stage("p = 29\nq = 4\nr = p // q")

    def test_unsupported_target(self):
        with self.assertRaises(TypeError):
            compile_first_stage("j, k = 10, 20")

    def test_unsupported_expression(self):
        with self.assertRaises(NotImplementedError):
            compile_first_stage("l = [7, 14, 21]")

    def test_while_loop_raises(self):
        with self.assertRaises(NotImplementedError):
            compile_first_stage("x = 1\nwhile x:\n    x = 0")

    def test_for_variable_range_raises(self):
        with self.assertRaises(TypeError):
            compile_first_stage(
                "from starisc import private\nn = private(0)\nfor i in range(n):\n    x = i"
            )

    def test_for_loop_index_in_expression(self):
        self.assertEqual(
            compile_first_stage("acc = 0\nfor i in range(3):\n    acc = acc + (i + 1)"),
            [
                Op("SET", "acc", "0"),
                Op("SET", "t0", "0"),
                Op("SET", "t1", "1"),
                Op("ADD", "t2", "t0", "t1"),
                Op("ADD", "acc", "acc", "t2"),
                Op("SET", "t3", "1"),
                Op("SET", "t4", "1"),
                Op("ADD", "t5", "t3", "t4"),
                Op("ADD", "acc", "acc", "t5"),
                Op("SET", "t6", "2"),
                Op("SET", "t7", "1"),
                Op("ADD", "t8", "t6", "t7"),
                Op("ADD", "acc", "acc", "t8"),
            ],
        )

    def test_for_loop_const_override_bound(self):
        self.assertEqual(
            compile_first_stage(
                'N = const("N")\nacc = 0\nfor i in range(N):\n    acc = acc + 1',
                {"N": 3},
            ),
            [
                Op("SET", "acc", "0"),
                Op("SET", "t0", "1"),
                Op("ADD", "acc", "acc", "t0"),
                Op("SET", "t1", "1"),
                Op("ADD", "acc", "acc", "t1"),
                Op("SET", "t2", "1"),
                Op("ADD", "acc", "acc", "t2"),
            ],
        )

    def test_named_const_in_expression(self):
        self.assertEqual(
            compile_first_stage('N = const("N")\nx = N + 1', {"N": 3}),
            [
                Op("SET", "t0", "3"),
                Op("SET", "t1", "1"),
                Op("ADD", "x", "t0", "t1"),
            ],
        )

    def test_branch_assignment_invalidates_inline_const(self):
        self.assertEqual(
            compile_first_stage(
                "from starisc import private\n"
                'N = const("N")\n'
                "flag = private(0)\n"
                "if flag:\n"
                "    N = 5\n"
                "x = N",
                {"N": 3},
            ),
            [
                Op("READ_PRIV", "flag", "0"),
                Op("LT", "t0", "r0", "flag"),
                Op("JZ", "t0", "1"),
                Op("SET", "N", "5"),
                Op("SET", "x", "N"),
            ],
        )

    def test_nested_for_same_name_restores_outer_index(self):
        self.assertEqual(
            compile_first_stage(
                "for i in range(2):\n    for i in range(2):\n        x = i\n    y = i"
            ),
            [
                Op("SET", "x", "0"),
                Op("SET", "x", "1"),
                Op("SET", "y", "0"),
                Op("SET", "x", "0"),
                Op("SET", "x", "1"),
                Op("SET", "y", "1"),
            ],
        )

    def test_missing_const_raises(self):
        with self.assertRaises(TypeError):
            compile_first_stage('N = const("N")')

    def test_and_condition_raises(self):
        with self.assertRaises(NotImplementedError):
            compile_first_stage("x = 1\ny = 2\nif x and y:\n    z = 3")

    def test_or_condition_raises(self):
        with self.assertRaises(NotImplementedError):
            compile_first_stage("x = 1\ny = 2\nif x or y:\n    z = 3")

    def test_list_condition_raises(self):
        with self.assertRaises(NotImplementedError):
            compile_first_stage("if [1, 2]:\n    x = 3")

    def test_call_condition_raises(self):
        with self.assertRaises(NotImplementedError):
            compile_first_stage("if foo():\n    x = 3")

    def test_import_raises(self):
        with self.assertRaises(NotImplementedError):
            compile_first_stage("import math")

    def test_func_raises(self):
        with self.assertRaises(NotImplementedError):
            compile_first_stage("def foo(x):\n    return x")


class TestBackendAllocation(unittest.TestCase):
    def test_allocator_uses_only_vm_registers(self):
        ops = optimize_ops(compile_first_stage(read("nested_deep.py")))
        allocation = allocate_registers(ops)

        for register in allocation.values():
            self.assertRegex(register, r"^r([1-9]|1[0-5])$")

    def test_interfering_names_do_not_share_a_register(self):
        ops = optimize_ops(compile_first_stage(read("nested_deep.py")))
        _, live_out = compute_liveness(ops)
        allocation = allocate_registers(ops)

        for op, live_after in zip(ops, live_out):
            if op.opcode == "ASSERT_EQ":
                continue
            for live_name in live_after:
                if live_name == op.dst:
                    continue
                self.assertNotEqual(allocation[op.dst], allocation[live_name])

    def test_rejects_more_than_fifteen_live_names(self):
        ops = [
            Op("SET", "a1", "1"),
            Op("SET", "a2", "2"),
            Op("SET", "a3", "3"),
            Op("SET", "a4", "4"),
            Op("SET", "a5", "5"),
            Op("SET", "a6", "6"),
            Op("SET", "a7", "7"),
            Op("SET", "a8", "8"),
            Op("SET", "a9", "9"),
            Op("SET", "a10", "10"),
            Op("SET", "a11", "11"),
            Op("SET", "a12", "12"),
            Op("SET", "a13", "13"),
            Op("SET", "a14", "14"),
            Op("SET", "a15", "15"),
            Op("SET", "a16", "16"),
            Op("ADD", "u1", "a1", "a2"),
            Op("ADD", "u2", "a3", "a4"),
            Op("ADD", "u3", "a5", "a6"),
            Op("ADD", "u4", "a7", "a8"),
            Op("ADD", "u5", "a9", "a10"),
            Op("ADD", "u6", "a11", "a12"),
            Op("ADD", "u7", "a13", "a14"),
            Op("ADD", "u8", "a15", "a16"),
        ]

        with self.assertRaisesRegex(ValueError, "more than 15"):
            allocate_registers(ops)


class TestBackendEmitter(unittest.TestCase):
    """
    super basic test
    """

    def test_emits_vm_format_exactly(self):
        self.assertEqual(
            emit_ops(
                [
                    Op("SET", "r3", "12"),
                    Op("ADD", "r5", "r3", "r4"),
                    Op("ASSERT_EQ", "r3", "r7"),
                ]
            ),
            "SET r3 12\nADD r5 r3 r4\nASSERT_EQ r3 r7",
        )

    def test_lowers_register_copy_to_add_with_r0(self):
        self.assertEqual(emit_ops([Op("SET", "r2", "r5")]), "ADD r2 r5 r0")

    def test_emits_jz(self):
        self.assertEqual(emit_ops([Op("JZ", "r0", "2")]), "JZ r0 2")


class TestBackendBranching(unittest.TestCase):
    def test_jz_uses_condition_and_has_no_def(self):
        op = Op("JZ", "cond", "3")
        self.assertEqual(op_uses(op), {"cond"})
        self.assertEqual(op_defs(op), set())

    def test_liveness_respects_branch_successors(self):
        ops = [
            Op("SET", "cond", "1"),
            Op("JZ", "cond", "1"),
            Op("SET", "x", "2"),
            Op("ASSERT_EQ", "x", "x"),
        ]
        _, live_out = compute_liveness(ops)
        self.assertEqual(live_out[1], {"x"})

    def test_optimizer_keeps_both_branch_results_live(self):
        program = read("lt_assert.py")
        compiled = compile_to_op(program)
        self.assertIn("JZ", compiled)
        self.assertEqual(compiled.count("SUB"), 2)
        self.assertIn("ASSERT_EQ", compiled)


class TestBackendPipeline(unittest.TestCase):
    def test_single_variable_pipeline(self):
        self.assertEqual(
            compile_to_op("x = 7\nassert x == x"),
            "SET r1 7\nASSERT_EQ r1 r1",
        )

    def test_all_temporaries_pipeline(self):
        self.assertEqual(
            compile_to_op("assert 2 + 3 == 5"),
            "SET r1 2\nSET r2 3\nADD r1 r1 r2\nSET r2 5\nASSERT_EQ r1 r2",
        )

    def test_copy_is_preserved_when_values_interfere(self):
        self.assertEqual(
            compile_to_op("a = 1\nd = a\nassert d == a"),
            "SET r1 1\nADD r2 r1 r0\nASSERT_EQ r2 r1",
        )

    def test_nested_program_emits_valid_lines(self):
        self.assertEqual(
            compile_to_op(read("nested_assert.py")),
            "SET r3 8\nSET r4 13\nSET r2 85\nSET r1 19\nMUL r3 r3 r4\nADD r1 r1 r2\nASSERT_EQ r3 r1",
        )

    def test_dead_assignments_are_removed(self):
        self.assertEqual(
            compile_to_op("x = 1\ny = 2\nassert x == x"), "SET r1 1\nASSERT_EQ r1 r1"
        )

    def test_dead_assignment_in_jump_span_preserves_offset(self):
        self.assertEqual(
            compile_to_op("x = 1\nif x < 2:\n    y = 3\nassert x == x"),
            "SET r1 1\nSET r2 2\nLT r2 r1 r2\nJZ r2 1\nSET r2 3\nASSERT_EQ r1 r1",
        )

    def test_coalesced_copy_in_jump_span_preserves_offset(self):
        self.assertEqual(
            compile_to_op("x = 1\nif x < 2:\n    y = x\nz = 9\nclaim(z)"),
            "SET r1 1\nSET r2 2\nLT r2 r1 r2\nJZ r2 1\nADD r1 r1 r0\nSET r1 9",
        )


class TestInputs(unittest.TestCase):
    def test_private_input_flattens(self):
        self.assertEqual(
            compile_first_stage("x = private(0)"),
            [Op("READ_PRIV", "x", "0")],
        )

    def test_public_input_flattens(self):
        self.assertEqual(
            compile_first_stage("x = public(0)"),
            [Op("READ_PUB", "x", "0")],
        )

    def test_multiple_slots(self):
        self.assertEqual(
            compile_first_stage("a = private(0)\nb = private(1)"),
            [Op("READ_PRIV", "a", "0"), Op("READ_PRIV", "b", "1")],
        )

    def test_input_feeds_arithmetic(self):
        ops = compile_first_stage("x = private(0)\ny = private(1)\nz = x + y")
        self.assertEqual(ops[-1], Op("ADD", "z", "x", "y"))

    def test_negative_slot_raises(self):
        with self.assertRaises((TypeError, NotImplementedError)):
            compile_first_stage("x = private(-1)")

    def test_non_int_slot_raises(self):
        with self.assertRaises(TypeError):
            compile_first_stage("x = private(1.5)")

    def test_private_input_emits_read_priv(self):
        self.assertEqual(
            compile_to_op("x = private(0)\ny = private(1)\nassert x == y"),
            "READ_PRIV r1 0\nREAD_PRIV r2 1\nASSERT_EQ r1 r2",
        )

    def test_public_input_emits_read_pub(self):
        self.assertEqual(
            compile_to_op("x = public(0)\ny = public(1)\nassert x == y"),
            "READ_PUB r1 0\nREAD_PUB r2 1\nASSERT_EQ r1 r2",
        )

    def test_mixed_inputs(self):
        src = "a = private(0)\nb = public(0)\nc = a + b\nassert c == c"
        op = compile_to_op(src)
        self.assertIn("READ_PRIV", op)
        self.assertIn("READ_PUB", op)
        self.assertIn("ADD", op)


class TestClaim(unittest.TestCase):
    def test_output_flattens(self):
        self.assertEqual(
            compile_first_stage("x = 5\nclaim(x)"),
            [Op("SET", "x", "5"), Op("CLAIM", "x")],
        )

    def test_output_prevents_dce(self):
        op = compile_to_op("x = private(0)\ny = private(1)\nz = x + y\nclaim(z)")
        self.assertIn("ADD", op)

    def test_output_emits_no_instruction(self):
        op = compile_to_op("x = private(0)\nclaim(x)")
        self.assertEqual(op, "READ_PRIV r1 0")

    def test_without_output_dce_removes(self):
        op = compile_to_op("x = private(0)\ny = private(1)\nz = x + y")
        self.assertEqual(op, "")


class TestForLoop(unittest.TestCase):
    def test_basic_unroll(self):
        ops = compile_first_stage("x = 0\nfor i in range(3):\n    x = x + 1\nclaim(x)")
        adds = [op for op in ops if op.opcode == "ADD"]
        self.assertEqual(len(adds), 3)

    def test_range_start_end(self):
        ops = compile_first_stage(
            "x = 0\nfor i in range(2, 5):\n    x = x + 1\nclaim(x)"
        )
        adds = [op for op in ops if op.opcode == "ADD"]
        self.assertEqual(len(adds), 3)

    def test_loop_var_in_private(self):
        ops = compile_first_stage("for i in range(3):\n    x = private(i)\nclaim(x)")
        reads = [op for op in ops if op.opcode == "READ_PRIV"]
        self.assertEqual(len(reads), 3)
        self.assertEqual([op.src1 for op in reads], ["0", "1", "2"])

    def test_loop_var_expr_in_private(self):
        ops = compile_first_stage(
            "for i in range(3):\n    x = private(i + 1)\nclaim(x)"
        )
        reads = [op for op in ops if op.opcode == "READ_PRIV"]
        self.assertEqual(len(reads), 3)
        self.assertEqual([op.src1 for op in reads], ["1", "2", "3"])

    def test_for_else_raises(self):
        with self.assertRaises(NotImplementedError):
            compile_first_stage("for i in range(3):\n    x = i\nelse:\n    x = 0")

    def test_for_list_raises(self):
        with self.assertRaises(TypeError):
            compile_first_stage("for i in [4, 5, 6]:\n    x = i")


if __name__ == "__main__":
    unittest.main()
