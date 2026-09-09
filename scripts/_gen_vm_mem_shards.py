#!/usr/bin/env python3
"""Generate densified VM mem op shards (session S arg)."""
from pathlib import Path

SH = Path(__file__).resolve().parents[1] / "self_host"

# Each shard: (suffix, fn, kab_body_lines inside function, extra_imports)
SHARDS = []

def shard(suffix, fn, body, imports=None):
    imps = ['import "self_host/emit_defs"', 'import "self_host/vm_s_stack"']
    if imports:
        imps.extend(imports)
    text = (
        f"// P6b: VM mem shard ({suffix}).\n"
        + "\n".join(imps)
        + f"\n\npub fn {fn}(S, name, arg, op, pool, globals, functions) {{\n"
        + body
        + "\n    return 0\n}\n"
    )
    (SH / f"vm_ops_mem_{suffix}.kab").write_text(text, encoding="utf-8", newline="\n")
    print("wrote", f"vm_ops_mem_{suffix}.kab")
    SHARDS.append((suffix, fn))


shard(
    "const_pop",
    "runOpMemConstPop",
    """\
    if name == OP_CONST {
        vPushS(S, poolVal(pool, arg))
        vBumpIpS(S)
        return 1
    }
    if name == OP_POP {
        vPopS(S)
        vBumpIpS(S)
        return 1
    }""",
    ['import "self_host/vm_s_pool"'],
)

shard(
    "local",
    "runOpMemLocal",
    """\
    if name == OP_LOAD_LOCAL {
        vPushS(S, S["locals"][arg])
        vBumpIpS(S)
        return 1
    }
    if name == OP_STORE_LOCAL {
        let v = vPopS(S)
        S["locals"][arg] = v
        vBumpIpS(S)
        return 1
    }""",
)

shard(
    "global",
    "runOpMemGlobal",
    """\
    if name == OP_LOAD_GLOBAL {
        vPushS(S, vResolveGlobalS(S, globals, arg))
        vBumpIpS(S)
        return 1
    }
    if name == OP_STORE_GLOBAL {
        let gname = globals[arg]
        let gv = vPopS(S)
        if gname == "this" || gname == "self" {
        } else {
            S["globals"][arg] = gv
        }
        vBumpIpS(S)
        return 1
    }""",
    ['import "self_host/vm_s_resolve"'],
)

shard(
    "array_push",
    "runOpMemArrayPush",
    """\
    if name == "array_push" {
        let item = vPopS(S)
        let arr = vPopS(S)
        let next = push(arr, item)
        vPushS(S, next)
        vPushS(S, len(next))
        vBumpIpS(S)
        return 1
    }""",
)

shard(
    "take",
    "runOpMemTake",
    """\
    if name == "take_local" {
        let arr = S["locals"][arg]
        S["locals"][arg] = undefined
        vPushS(S, arr)
        vBumpIpS(S)
        return 1
    }
    if name == "take_global" {
        let arr = vResolveGlobalS(S, globals, arg)
        S["globals"][arg] = undefined
        vPushS(S, arr)
        vBumpIpS(S)
        return 1
    }""",
    ['import "self_host/vm_s_resolve"'],
)

shard(
    "array_push_bind",
    "runOpMemArrayPushBind",
    """\
    if name == "array_push_local" {
        let item = vPopS(S)
        let arr = vPopS(S)
        let next = bytecode_array_push(arr, item)
        S["locals"][arg] = next
        vPushS(S, len(next))
        vBumpIpS(S)
        return 1
    }
    if name == "array_push_global" {
        let item = vPopS(S)
        let arr = vPopS(S)
        let next = bytecode_array_push(arr, item)
        S["globals"][arg] = next
        vPushS(S, len(next))
        vBumpIpS(S)
        return 1
    }""",
)

shard(
    "array_pop_bind",
    "runOpMemArrayPopBind",
    """\
    if name == "array_pop_local" {
        let arr = S["locals"][arg]
        S["locals"][arg] = pop(arr)
        vPushS(S, null)
        vBumpIpS(S)
        return 1
    }
    if name == "array_pop_global" {
        let arr = vResolveGlobalS(S, globals, arg)
        S["globals"][arg] = pop(arr)
        vPushS(S, null)
        vBumpIpS(S)
        return 1
    }""",
    ['import "self_host/vm_s_resolve"'],
)

shard(
    "acc_len",
    "runOpMemAccLen",
    """\
    if name == "acc_add_local" {
        let rhs = vPopS(S)
        S["locals"][arg] = S["locals"][arg] + rhs
        vBumpIpS(S)
        return 1
    }
    if name == "acc_add_global" {
        let rhs = vPopS(S)
        let cur = vResolveGlobalS(S, globals, arg)
        S["globals"][arg] = cur + rhs
        vBumpIpS(S)
        return 1
    }
    if name == "len_local" {
        vPushS(S, len(S["locals"][arg]))
        vBumpIpS(S)
        return 1
    }
    if name == "len_global" {
        vPushS(S, len(vResolveGlobalS(S, globals, arg)))
        vBumpIpS(S)
        return 1
    }""",
    ['import "self_host/vm_s_resolve"'],
)

# acc_len has 4 ops - might be OVER; split if needed
shard(
    "index_get",
    "runOpMemIndexGet",
    """\
    if name == "index_get_local" {
        let idx = vPopS(S)
        vPushS(S, S["locals"][arg][idx])
        vBumpIpS(S)
        return 1
    }
    if name == "index_get_global" {
        let idx = vPopS(S)
        vPushS(S, vResolveGlobalS(S, globals, arg)[idx])
        vBumpIpS(S)
        return 1
    }""",
    ['import "self_host/vm_s_resolve"'],
)

# Half facades: pairs of shards
HALVES = [
    ("a", "runOpMemA", ["runOpMemConstPop", "runOpMemLocal"], ["const_pop", "local"]),
    ("b", "runOpMemB", ["runOpMemGlobal", "runOpMemArrayPush"], ["global", "array_push"]),
    ("c", "runOpMemC", ["runOpMemTake", "runOpMemArrayPushBind"], ["take", "array_push_bind"]),
    ("d", "runOpMemD", ["runOpMemArrayPopBind", "runOpMemAccLen"], ["array_pop_bind", "acc_len"]),
    ("e", "runOpMemE", ["runOpMemIndexGet"], ["index_get"]),
]

for suffix, fn, calls, imports in HALVES:
    imps = "\n".join(f'import "self_host/vm_ops_mem_{s}"' for s in imports)
    if len(calls) == 1:
        body = f"    return {calls[0]}(S, name, arg, op, pool, globals, functions)\n"
    else:
        body = (
            f"    let st = {calls[0]}(S, name, arg, op, pool, globals, functions)\n"
            f"    if st != 0 {{\n        return st\n    }}\n"
            f"    return {calls[1]}(S, name, arg, op, pool, globals, functions)\n"
        )
    text = f"// P6b: VM mem half-facade {suffix}.\n{imps}\n\npub fn {fn}(S, name, arg, op, pool, globals, functions) {{\n{body}}}\n"
    (SH / f"vm_ops_mem_{suffix}.kab").write_text(text, encoding="utf-8", newline="\n")
    print("wrote", f"vm_ops_mem_{suffix}.kab")

print("done", len(SHARDS), "shards")
