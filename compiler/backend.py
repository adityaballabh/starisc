from collections import defaultdict
import re

from .constants import (
    FIRST_ALLOC_REG,
    LAST_ALLOC_REG,
    OP_ADD,
    OP_ASSERT_EQ,
    OP_CLAIM,
    OP_JZ,
    OP_READ_PRIV,
    OP_READ_PUB,
    OP_SET,
    ZERO_REGISTER,
)
from .op import Op


def is_immediate(value: str | None) -> bool:
    return value is None or value.lstrip("-").isdigit()


def is_register(value: str | None) -> bool:
    return value is not None and re.fullmatch(r"r([0-9]|1[0-5])", value) is not None


def is_name(value: str | None) -> bool:
    return value is not None and not is_immediate(value) and not is_register(value)


def op_uses(op: Op) -> set[str]:
    if op.opcode == OP_CLAIM:
        return {op.dst} if is_name(op.dst) else set()
    if op.opcode == OP_ASSERT_EQ:
        return {value for value in (op.dst, op.src1) if is_name(value)}
    if op.opcode == OP_JZ:
        return {op.dst} if is_name(op.dst) else set()
    return {value for value in (op.src1, op.src2) if is_name(value)}


def op_defs(op: Op) -> set[str]:
    if op.opcode in {OP_ASSERT_EQ, OP_JZ, OP_CLAIM}:
        return set()
    return {op.dst} if is_name(op.dst) else set()


def compute_liveness(ops: list[Op]) -> tuple[list[set[str]], list[set[str]]]:
    live_in = [set() for _ in ops]
    live_out = [set() for _ in ops]

    for index in range(len(ops) - 1, -1, -1):
        op = ops[index]
        successors = []
        if index + 1 <= len(ops):
            successors.append(index + 1)
        if op.opcode == OP_JZ:
            successors.append(index + 1 + int(op.src1))

        next_live = set()
        for successor in successors:
            if successor < len(ops):
                next_live |= live_in[successor]

        live_out[index] = next_live
        live_in[index] = op_uses(op) | (live_out[index] - op_defs(op))

    return live_in, live_out


def eliminate_dead_assignments(ops: list[Op]) -> list[Op]:
    optimized = list(ops)

    while True:
        _, live_out = compute_liveness(optimized)
        next_ops = []
        changed = False
        protected_indices = jump_protected_indices(optimized)

        for index, (op, live_after) in enumerate(zip(optimized, live_out)):
            defs = op_defs(op)
            if defs and defs.isdisjoint(live_after) and index not in protected_indices:
                changed = True
                continue
            next_ops.append(op)

        if not changed:
            return optimized
        optimized = next_ops


def jump_protected_indices(ops: list[Op]) -> set[int]:
    # keep jump spans stable after offsets are emitted
    indices = set()
    for index, op in enumerate(ops):
        if op.opcode != OP_JZ:
            continue
        target = index + 1 + int(op.src1)
        indices.update(range(index + 1, min(target, len(ops))))
    return indices


def build_interference_graph(ops: list[Op]) -> dict[str, set[str]]:
    graph: dict[str, set[str]] = defaultdict(set)
    _, live_out = compute_liveness(ops)

    for op in ops:
        for name in op_uses(op) | op_defs(op):
            graph[name]

    for op, live_after in zip(ops, live_out):
        defs = op_defs(op)
        for defined in defs:
            for live_name in live_after:
                if live_name == defined:
                    continue
                graph[defined].add(live_name)
                graph[live_name].add(defined)

    return dict(graph)


class DisjointSets:
    def __init__(self, names: set[str]):
        self.parent = {name: name for name in names}
        self.members = {name: {name} for name in names}
        self.neighbors = {name: set() for name in names}

    def find(self, name: str) -> str:
        parent = self.parent[name]
        if parent != name:
            self.parent[name] = self.find(parent)
        return self.parent[name]

    def union(self, left: str, right: str) -> str:
        left = self.find(left)
        right = self.find(right)
        if left == right:
            return left
        if len(self.members[left]) < len(self.members[right]):
            left, right = right, left

        self.parent[right] = left
        self.members[left] |= self.members[right]
        del self.members[right]

        merged_neighbors = (self.neighbors[left] | self.neighbors[right]) - {
            left,
            right,
        }
        for neighbor in list(merged_neighbors):
            self.neighbors[neighbor].discard(left)
            self.neighbors[neighbor].discard(right)
            self.neighbors[neighbor].add(left)
        self.neighbors[left] = merged_neighbors
        del self.neighbors[right]
        return left


def coalesce_copies(ops: list[Op]) -> list[Op]:
    graph = build_interference_graph(ops)
    dsu = DisjointSets(set(graph))
    protected_indices = jump_protected_indices(ops)

    for name, neighbors in graph.items():
        dsu.neighbors[name] = set(neighbors)

    for op in ops:
        if op.opcode != OP_SET or not is_name(op.src1):
            continue
        src = dsu.find(op.src1)
        dst = dsu.find(op.dst)
        if src == dst or dst in dsu.neighbors[src]:
            continue
        dsu.union(src, dst)

    replacement = {}
    for name in graph:
        group = dsu.find(name)
        replacement[name] = min(dsu.members[group])

    rewritten = []
    for index, op in enumerate(ops):
        dst = replacement.get(op.dst, op.dst)
        src1 = replacement.get(op.src1, op.src1)
        src2 = replacement.get(op.src2, op.src2)
        if (
            op.opcode == OP_SET
            and is_name(src1)
            and dst == src1
            and index not in protected_indices
        ):
            continue
        rewritten.append(Op(op.opcode, dst, src1, src2))

    return rewritten


def optimize_ops(ops: list[Op]) -> list[Op]:
    optimized = eliminate_dead_assignments(ops)
    optimized = coalesce_copies(optimized)
    return eliminate_dead_assignments(optimized)


def allocate_registers(ops: list[Op]) -> dict[str, str]:
    graph = build_interference_graph(ops)
    order = sorted(graph, key=lambda name: (-len(graph[name]), name))
    allocation: dict[str, str] = {}

    for name in order:
        used = {
            allocation[neighbor] for neighbor in graph[name] if neighbor in allocation
        }
        register = next(
            (
                f"r{idx}"
                for idx in range(FIRST_ALLOC_REG, LAST_ALLOC_REG + 1)
                if f"r{idx}" not in used
            ),
            None,
        )
        if register is None:
            raise ValueError("program requires more than 15 live registers")
        allocation[name] = register

    return allocation


def apply_allocation(ops: list[Op], allocation: dict[str, str]) -> list[Op]:
    rewritten = []
    for op in ops:
        if op.opcode == OP_CLAIM:
            continue
        if op.opcode == OP_JZ:
            rewritten.append(Op(OP_JZ, allocation.get(op.dst, op.dst), op.src1))
            continue
        dst = allocation.get(op.dst, op.dst)
        src1 = allocation.get(op.src1, op.src1)
        src2 = allocation.get(op.src2, op.src2)
        rewritten.append(Op(op.opcode, dst, src1, src2))
    return rewritten


def emit_ops(ops: list[Op]) -> str:
    lines = []

    for op in ops:
        if op.opcode in (OP_READ_PRIV, OP_READ_PUB):
            lines.append(f"{op.opcode} {op.dst} {op.src1}")
            continue
        if op.opcode == OP_SET:
            if is_name(op.src1) or is_register(op.src1):
                lines.append(f"{OP_ADD} {op.dst} {op.src1} {ZERO_REGISTER}")
            else:
                lines.append(f"{OP_SET} {op.dst} {op.src1}")
            continue
        if op.opcode == OP_ASSERT_EQ:
            lines.append(f"{OP_ASSERT_EQ} {op.dst} {op.src1}")
            continue
        if op.opcode == OP_JZ:
            lines.append(f"{OP_JZ} {op.dst} {op.src1}")
            continue
        lines.append(f"{op.opcode} {op.dst} {op.src1} {op.src2}")
    return "\n".join(lines)


class Backend:
    def run(self, ops: list[Op]) -> str:
        optimized = optimize_ops(ops)
        allocation = allocate_registers(optimized)
        allocated = apply_allocation(optimized, allocation)
        return emit_ops(allocated)

    def run_with_symbols(self, ops: list[Op]) -> tuple[str, dict[str, str]]:
        optimized = optimize_ops(ops)
        allocation = allocate_registers(optimized)
        allocated = apply_allocation(optimized, allocation)
        symbols = {
            name: reg for name, reg in allocation.items() if not name.startswith("t")
        }
        return emit_ops(allocated), symbols
