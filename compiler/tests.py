import unittest
from pathlib import Path
from .pipeline import compiler
from .op import Op


def read(name):
    return (Path(__file__).parent / "examples" / name).read_text()


class TestSet(unittest.TestCase):
    def test_copy(self):
        self.assertEqual(compiler("a = 88\nb = a"), [
            Op("SET", "a", "88"), 
            Op("SET", "b", "a"),
        ])

    def test_self_assign(self):
        self.assertEqual(compiler("c = 23\nc = c"), [Op("SET", "c", "23")])


class TestArithmetic(unittest.TestCase):
    def test_add(self):
        self.assertEqual(compiler("p = 45\nq = 22\nr = p + q"), [
            Op("SET", "p", "45"),
            Op("SET", "q", "22"),
            Op("ADD", "r", "p", "q"),
        ])

    def test_sub(self):
        self.assertEqual(compiler("c = 56\nd = 45\na = d - c"), [
            Op("SET", "c", "56"),
            Op("SET", "d", "45"),
            Op("SUB", "a", "d", "c"),
        ])

    def test_mul(self):
        self.assertEqual(compiler("x = 23\ny= 7\nz = x * y"), [
            Op("SET", "x", "23"),
            Op("SET", "y", "7"),
            Op("MUL", "z", "x", "y"),
        ])

    def test_mod(self):
        self.assertEqual(compiler("n = 38\nm = 7\nr = n % m"), [
            Op("SET", "n", "38"),
            Op("SET", "m", "7"),
            Op("MOD", "r", "n", "m"),
        ])


class TestComparisons(unittest.TestCase):
    def test_lt(self):
        self.assertEqual(compiler("r = 24\ns = 91\nres = r < s"), [
            Op("SET", "r", "24"),
            Op("SET", "s", "91"),
            Op("LT", "res", "r", "s"),
        ])

    def test_gt(self):
        self.assertEqual(compiler("r = 45\ns = 16\nres = r > s"), [
            Op("SET", "r", "45"),
            Op("SET", "s", "16"),
            Op("LT", "res", "s", "r"),
        ])

    def test_gte(self):
        self.assertEqual(compiler("a = 23\nb = 52\nc = a >= b"), [
            Op("SET", "a", "23"),
            Op("SET", "b", "52"),
            Op("LT", "t0", "a", "b"),
            Op("SET", "t1", "1"),
            Op("SUB", "c", "t1", "t0"),
        ])

    def test_lte(self):
        self.assertEqual(compiler("p = 19\nq = 21\nr = p <= q"), [
            Op("SET", "p", "19"),
            Op("SET", "q", "21"),
            Op("LT", "t0", "q", "p"),
            Op("SET", "t1", "1"),
            Op("SUB", "r", "t1", "t0"),
        ])


class TestPow(unittest.TestCase):
    def test_pow_zero(self):
        self.assertEqual(compiler("g = 37\nres = g ** 0"), [
            Op("SET", "g", "37"),
            Op("SET", "res", "1"),
        ])

    def test_pow_one(self):
        self.assertEqual(compiler("g = 52\nres = g ** 1"), [
            Op("SET", "g", "52"),
            Op("SET", "res", "g"),
        ])

    def test_pow_five(self):
        self.assertEqual(compiler("a = 7\nb = a ** 5"), [
            Op("SET", "a", "7"),
            Op("MUL", "t0", "a", "a"),
            Op("MUL", "t1", "t0", "t0"),
            Op("MUL", "b", "t1", "a"),
        ])

    def test_pow_neg_raises(self):
        with self.assertRaises(TypeError):
            compiler("c = 11\ne = c ** -1")

    def test_pow_var_raises(self):
        with self.assertRaises(TypeError):
            compiler("p = 2\nq = 8\nres = p ** q")


class TestAssert(unittest.TestCase):
    def test_assert_vars(self):
        self.assertEqual(compiler("i = 43\nj = 43\nassert i == j"), [
            Op("SET", "i", "43"),
            Op("SET", "j", "43"),
            Op("ASSERT_EQ", "i", "j"),
        ])

    def test_assert_const(self):
        self.assertEqual(compiler("v = 77\nassert v == 77"), [
            Op("SET", "v", "77"),
            Op("SET", "t0", "77"),
            Op("ASSERT_EQ", "v", "t0"),
        ])

    def test_assert_nested(self):
        self.assertEqual(compiler("a = 13\nb = 4\nd = 52\nassert a * b == d"), [
            Op("SET", "a", "13"),
            Op("SET", "b", "4"),
            Op("SET", "d", "52"),
            Op("MUL", "t0", "a", "b"),
            Op("ASSERT_EQ", "t0", "d"),
        ])

    def test_assert_non_compare_raises(self):
        with self.assertRaises(NotImplementedError):
            compiler("assert False")


class TestNested(unittest.TestCase):
    def test_nested_deep(self):
        self.assertEqual(compiler(read("nested_deep.py")), [
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
        ])

    def test_nested_assert(self):
        self.assertEqual(compiler(read("nested_assert.py")), [
            Op("SET", "x", "8"),
            Op("SET", "y", "13"),
            Op("SET", "z", "85"),
            Op("SET", "w", "19"),
            Op("MUL", "t0", "x", "y"),
            Op("ADD", "t1", "w", "z"),
            Op("ASSERT_EQ", "t0", "t1"),
        ])

    def test_nested_precedence(self):
        self.assertEqual(compiler(read("nested_precedence.py")), [
            Op("SET", "p", "13"),
            Op("SET", "q", "6"),
            Op("SET", "a", "53"),
            Op("SET", "b", "25"),
            Op("MUL", "t0", "p", "q"),
            Op("MOD", "t1", "a", "b"),
            Op("ADD", "t2", "t0", "t1"),
            Op("SET", "t3", "8"),
            Op("ADD", "res", "t2", "t3"),
        ])


class TestUnsupported(unittest.TestCase):
    def test_unsupported_operator(self):
        with self.assertRaises(TypeError):
            compiler("p = 29\nq = 4\nr = p // q")

    def test_unsupported_target(self):
        with self.assertRaises(TypeError):
            compiler("j, k = 10, 20")

    def test_unsupported_expression(self):
        with self.assertRaises(NotImplementedError):
            compiler("l = [7, 14, 21]")

    def test_loop_raises(self):
        with self.assertRaises(NotImplementedError):
            compiler("b = 1\nfor i in range(3):\n   b = b * 3")

    def test_if_raises(self):
        with self.assertRaises(NotImplementedError):
            compiler("c = 12\nif c > 10:\n    c = c - 5")

    def test_import_raises(self):
        with self.assertRaises(NotImplementedError):
            compiler("import math")

    def test_func_raises(self):
        with self.assertRaises(NotImplementedError):
            compiler("def foo(x):\n    return x")


if __name__ == "__main__":
    unittest.main()
