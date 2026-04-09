import unittest
from pathlib import Path

from .backend import allocate_registers, compute_liveness, emit_ops, optimize_ops
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

    def test_pow_var_raises(self):
        with self.assertRaises(TypeError):
            compile_first_stage("p = 2\nq = p + 15\nres = p ** q")

    def test_pow_reassigned_name_raises(self):
        with self.assertRaises(TypeError):
            compile_first_stage("e = 5\ne = e + 1\na = 7\nres = a ** e")


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

    def test_loop_raises(self):
        with self.assertRaises(NotImplementedError):
            compile_first_stage("b = 1\nfor i in range(3):\n   b = b * 3")

    def test_if_raises(self):
        with self.assertRaises(NotImplementedError):
            compile_first_stage("c = 12\nif c > 10:\n    c = c - 5")

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


if __name__ == "__main__":
    unittest.main()
