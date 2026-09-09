#!/usr/bin/env python3
"""Generate VM arith shards that mutate explicit session object S."""
from pathlib import Path

SH = Path(__file__).resolve().parents[1] / "self_host"

SHARDS = [
    ("add_sub", "runOpArithAddSub", [("OP_ADD", "binary", "a + b"), ("OP_SUB", "binary", "a - b")]),
    ("mul_div", "runOpArithMulDiv", [("OP_MUL", "binary", "a * b"), ("OP_DIV", "binary", "a / b")]),
    ("mod_pow", "runOpArithModPow", [("OP_MOD", "binary", "a % b"), ("OP_POW", "binary", "a ** b")]),
    ("neg", "runOpArithNeg", [("OP_NEG", "unary_neg", None)]),
    ("bit_and_or", "runOpArithBitAndOr", [('"bit_and"', "binary", "a & b"), ('"bit_or"', "binary", "a | b")]),
    ("bit_xor_not", "runOpArithBitXorNot", [('"bit_xor"', "binary", "a ^ b"), ('"bit_not"', "unary_not", None)]),
    ("shift", "runOpArithShift", [('"shl"', "binary", "a << b"), ('"shr"', "binary", "a >> b")]),
    ("ushr", "runOpArithUshr", [('"ushr"', "binary", "a >>> b")]),
]


def emit_op(op, kind, expr):
    lines = [f"    if name == {op} {{"]
    if kind == "binary":
        lines += [
            "        let b = vPopS(S)",
            "        let a = vPopS(S)",
            f"        vPushS(S, {expr})",
        ]
    elif kind == "unary_neg":
        lines += ["        let v = vPopS(S)", "        vPushS(S, 0 - v)"]
    elif kind == "unary_not":
        lines += ["        let v = vPopS(S)", "        vPushS(S, ~v)"]
    lines += ["        vBumpIpS(S)", "        return 1", "    }"]
    return "\n".join(lines)


for suffix, fn, ops in SHARDS:
    body = "\n".join(emit_op(*t) for t in ops)
    text = f"""\
// P6b: VM arith shard ({suffix}) — mutates session S (arg AccAdd).
import "self_host/emit_defs"
import "self_host/vm_s_stack"

pub fn {fn}(S, name, arg, op, pool, globals, functions) {{
{body}
    return 0
}}
"""
    (SH / f"vm_ops_arith_{suffix}.kab").write_text(text, encoding="utf-8", newline="\n")
    print("wrote", f"vm_ops_arith_{suffix}.kab")

HALVES = [
    ("a", "runOpArithA", ["runOpArithAddSub", "runOpArithMulDiv"], ["add_sub", "mul_div"]),
    ("b", "runOpArithB", ["runOpArithModPow", "runOpArithNeg"], ["mod_pow", "neg"]),
    ("c", "runOpArithC", ["runOpArithBitAndOr", "runOpArithBitXorNot"], ["bit_and_or", "bit_xor_not"]),
    ("d", "runOpArithD", ["runOpArithShift", "runOpArithUshr"], ["shift", "ushr"]),
]
for suffix, fn, calls, imports in HALVES:
    imps = "\n".join(f'import "self_host/vm_ops_arith_{s}"' for s in imports)
    text = f"""\
// P6b: VM arith half-facade {suffix}.
{imps}

pub fn {fn}(S, name, arg, op, pool, globals, functions) {{
    let st = {calls[0]}(S, name, arg, op, pool, globals, functions)
    if st != 0 {{
        return st
    }}
    return {calls[1]}(S, name, arg, op, pool, globals, functions)
}}
"""
    (SH / f"vm_ops_arith_{suffix}.kab").write_text(text, encoding="utf-8", newline="\n")
    print("wrote", f"vm_ops_arith_{suffix}.kab")

# No top-level 4-call facade (OVER budget) — body calls A–D directly.
print("done")
