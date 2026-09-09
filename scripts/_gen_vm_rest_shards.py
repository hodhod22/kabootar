#!/usr/bin/env python3
"""Generate VM cmp/data/jump/ctrl shards (S-arg)."""
from pathlib import Path

SH = Path(__file__).resolve().parents[1] / "self_host"


def write(name: str, body: str) -> None:
    (SH / name).write_text(body, encoding="utf-8", newline="\n")
    print("wrote", name)


# ---- CMP ----
write(
    "vm_ops_cmp_eq.kab",
    """\
// P6b: VM cmp == / !=
import "self_host/emit_defs"
import "self_host/vm_s_stack"

pub fn runOpCmpEq(S, name, arg, op, pool, globals, functions) {
    if name == OP_EQ {
        let b = vPopS(S)
        let a = vPopS(S)
        vPushS(S, a == b)
        vBumpIpS(S)
        return 1
    }
    if name == OP_NE {
        let b = vPopS(S)
        let a = vPopS(S)
        vPushS(S, a != b)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)

write(
    "vm_ops_cmp_ord.kab",
    """\
// P6b: VM cmp < > 
import "self_host/emit_defs"
import "self_host/vm_s_stack"

pub fn runOpCmpOrd(S, name, arg, op, pool, globals, functions) {
    if name == OP_LT {
        let b = vPopS(S)
        let a = vPopS(S)
        vPushS(S, a < b)
        vBumpIpS(S)
        return 1
    }
    if name == OP_GT {
        let b = vPopS(S)
        let a = vPopS(S)
        vPushS(S, a > b)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)

write(
    "vm_ops_cmp_ord2.kab",
    """\
// P6b: VM cmp <= >=
import "self_host/emit_defs"
import "self_host/vm_s_stack"

pub fn runOpCmpOrd2(S, name, arg, op, pool, globals, functions) {
    if name == OP_LE {
        let b = vPopS(S)
        let a = vPopS(S)
        vPushS(S, a <= b)
        vBumpIpS(S)
        return 1
    }
    if name == OP_GE {
        let b = vPopS(S)
        let a = vPopS(S)
        vPushS(S, a >= b)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)

write(
    "vm_ops_cmp_logic.kab",
    """\
// P6b: VM cmp and / or
import "self_host/emit_defs"
import "self_host/vm_s_stack"
import "self_host/vm_s_truthy"

pub fn runOpCmpLogic(S, name, arg, op, pool, globals, functions) {
    if name == OP_AND {
        let b = vPopS(S)
        let a = vPopS(S)
        if vTruthy(a) {
            vPushS(S, b)
        } else {
            vPushS(S, a)
        }
        vBumpIpS(S)
        return 1
    }
    if name == OP_OR {
        let b = vPopS(S)
        let a = vPopS(S)
        if vTruthy(a) {
            vPushS(S, a)
        } else {
            vPushS(S, b)
        }
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)

write(
    "vm_ops_cmp_not_member.kab",
    """\
// P6b: VM cmp not / get_member
import "self_host/emit_defs"
import "self_host/vm_s_stack"
import "self_host/vm_s_truthy"
import "self_host/vm_s_pool_key"
import "self_host/vm_s_member"

pub fn runOpCmpNotMember(S, name, arg, op, pool, globals, functions) {
    if name == OP_NOT {
        let v = vPopS(S)
        vPushS(S, !vTruthy(v))
        vBumpIpS(S)
        return 1
    }
    if name == OP_GET_MEMBER {
        let key = poolKey(pool, arg)
        let obj = vPopS(S)
        vPushS(S, vMemberGetS(S, obj, key))
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)

# halves a-c; new_instance stays in body
write(
    "vm_ops_cmp_a.kab",
    """\
import "self_host/vm_ops_cmp_eq"
import "self_host/vm_ops_cmp_ord"

pub fn runOpCmpA(S, name, arg, op, pool, globals, functions) {
    let st = runOpCmpEq(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpCmpOrd(S, name, arg, op, pool, globals, functions)
}
""",
)
write(
    "vm_ops_cmp_b.kab",
    """\
import "self_host/vm_ops_cmp_ord2"
import "self_host/vm_ops_cmp_logic"

pub fn runOpCmpB(S, name, arg, op, pool, globals, functions) {
    let st = runOpCmpOrd2(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpCmpLogic(S, name, arg, op, pool, globals, functions)
}
""",
)
write(
    "vm_ops_cmp_c.kab",
    """\
import "self_host/vm_ops_cmp_not_member"

pub fn runOpCmpC(S, name, arg, op, pool, globals, functions) {
    return runOpCmpNotMember(S, name, arg, op, pool, globals, functions)
}
""",
)

# ---- DATA ----
write(
    "vm_ops_data_make.kab",
    """\
import "self_host/emit_defs"
import "self_host/vm_s_stack"

pub fn runOpDataMake(S, name, arg, op, pool, globals, functions) {
    if name == OP_MAKE_OBJECT {
        let n = arg
        let map = {}
        let ci = 0
        while ci < n {
            let key = vPopS(S)
            let val = vPopS(S)
            map["" + key] = val
            ci = ci + 1
        }
        vPushS(S, map)
        vBumpIpS(S)
        return 1
    }
    if name == OP_MAKE_ARRAY {
        let n = arg
        let raw = []
        let ai = 0
        while ai < n {
            raw = push(raw, vPopS(S))
            ai = ai + 1
        }
        let arr = []
        let oi = n - 1
        while oi >= 0 {
            arr = push(arr, raw[oi])
            oi = oi - 1
        }
        vPushS(S, arr)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)

write(
    "vm_ops_data_index.kab",
    """\
import "self_host/emit_defs"
import "self_host/vm_s_stack"
import "self_host/vm_s_member"
import "self_host/vm_s_this"

pub fn runOpDataIndex(S, name, arg, op, pool, globals, functions) {
    if name == OP_INDEX_GET {
        let idx = vPopS(S)
        let container = vPopS(S)
        vPushS(S, vIndexGet(container, idx))
        vBumpIpS(S)
        return 1
    }
    if name == OP_INDEX_SET {
        let val = vPopS(S)
        let idx = vPopS(S)
        let container = vPopS(S)
        let asThis = vIsCurrentThisS(S, container)
        let updated = vIndexSet(container, idx, val)
        if asThis {
            vWriteThisS(S, updated)
        }
        vPushS(S, updated)
        vPushS(S, val)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)

write(
    "vm_ops_data_set_member.kab",
    """\
import "self_host/emit_defs"
import "self_host/vm_s_stack"
import "self_host/vm_s_pool_key"
import "self_host/vm_s_member"
import "self_host/vm_s_this"

pub fn runOpDataSetMember(S, name, arg, op, pool, globals, functions) {
    if name == OP_SET_MEMBER {
        let key = poolKey(pool, arg)
        let val = vPopS(S)
        let obj = vPopS(S)
        let asThis = vIsCurrentThisS(S, obj)
        let updated = vMemberSet(obj, key, val)
        if asThis {
            vWriteThisS(S, updated)
        }
        vPushS(S, updated)
        vPushS(S, val)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)

write(
    "vm_ops_data_stack.kab",
    """\
import "self_host/vm_s_stack"

pub fn runOpDataStack(S, name, arg, op, pool, globals, functions) {
    if name == "dup" {
        if len(S["stack"]) == 0 {
            throw "vm stack underflow"
        }
        vPushS(S, S["stack"][len(S["stack"]) - 1])
        vBumpIpS(S)
        return 1
    }
    if name == "swap" {
        let a = vPopS(S)
        let b = vPopS(S)
        vPushS(S, a)
        vPushS(S, b)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)

write(
    "vm_ops_data_len_concat.kab",
    """\
import "self_host/vm_s_stack"

pub fn runOpDataLenConcat(S, name, arg, op, pool, globals, functions) {
    if name == "get_length" {
        let v = vPopS(S)
        vPushS(S, len(v))
        vBumpIpS(S)
        return 1
    }
    if name == "concat_array" {
        let right = vPopS(S)
        let left = vPopS(S)
        let out = left
        let ci = 0
        while ci < len(right) {
            out = push(out, right[ci])
            ci = ci + 1
        }
        vPushS(S, out)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)

write(
    "vm_ops_data_merge.kab",
    """\
import "self_host/vm_s_stack"

pub fn runOpDataMerge(S, name, arg, op, pool, globals, functions) {
    if name == "merge_object" {
        let right = vPopS(S)
        let left = vPopS(S)
        let keys = object_keys(right)
        let ki = 0
        while ki < len(keys) {
            let k = keys[ki]
            left[k] = right[k]
            ki = ki + 1
        }
        vPushS(S, left)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)

write(
    "vm_ops_data_result.kab",
    """\
import "self_host/vm_s_stack"

pub fn runOpDataResult(S, name, arg, op, pool, globals, functions) {
    if name == "make_ok" {
        vPushS(S, { "vmResult": "ok", "v": vPopS(S) })
        vBumpIpS(S)
        return 1
    }
    if name == "make_err" {
        vPushS(S, { "vmResult": "err", "v": vPopS(S) })
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)

write(
    "vm_ops_data_option.kab",
    """\
import "self_host/vm_s_stack"
import "self_host/vm_s_caps"

pub fn runOpDataOption(S, name, arg, op, pool, globals, functions) {
    if name == "make_some" {
        vPushS(S, { "vmOption": "some", "v": vPopS(S) })
        vBumpIpS(S)
        return 1
    }
    if name == "make_none" {
        vPushS(S, { "vmOption": "none" })
        vBumpIpS(S)
        return 1
    }
    if name == "make_arrow_fn" {
        vPushS(S, { "vmArrow": arg, "caps": vSnapCapsS(S) })
        vBumpIpS(S)
        return 1
    }
    if name == "iterator_step_in_place" {
        let it = vPopS(S)
        let pair = bytecode_iterator_step_in_place(it)
        vPushS(S, pair[0])
        vPushS(S, pair[1])
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)

# data halves — option has 4 ops may be OVER; split later if needed
for suf, fn, calls, imps in [
    ("a", "runOpDataA", ["runOpDataMake", "runOpDataIndex"], ["make", "index"]),
    ("b", "runOpDataB", ["runOpDataSetMember", "runOpDataStack"], ["set_member", "stack"]),
    ("c", "runOpDataC", ["runOpDataLenConcat", "runOpDataMerge"], ["len_concat", "merge"]),
    ("d", "runOpDataD", ["runOpDataResult", "runOpDataOption"], ["result", "option"]),
]:
    imps_s = "\n".join(f'import "self_host/vm_ops_data_{s}"' for s in imps)
    write(
        f"vm_ops_data_{suf}.kab",
        f"""\
{imps_s}

pub fn {fn}(S, name, arg, op, pool, globals, functions) {{
    let st = {calls[0]}(S, name, arg, op, pool, globals, functions)
    if st != 0 {{
        return st
    }}
    return {calls[1]}(S, name, arg, op, pool, globals, functions)
}}
""",
    )

# ---- JUMP ----
write(
    "vm_ops_jump_basic.kab",
    """\
import "self_host/emit_defs"
import "self_host/vm_s_stack"
import "self_host/vm_s_truthy"

pub fn runOpJumpBasic(S, name, arg, op, pool, globals, functions) {
    if name == OP_JUMP {
        S["ip"] = S["ip"] + 1 + arg
        return 1
    }
    if name == OP_JUMP_IF_FALSE {
        let v = vPopS(S)
        if !vTruthy(v) {
            S["ip"] = S["ip"] + 1 + arg
            return 1
        }
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)

write(
    "vm_ops_jump_true.kab",
    """\
import "self_host/emit_defs"
import "self_host/vm_s_stack"
import "self_host/vm_s_truthy"

pub fn runOpJumpTrue(S, name, arg, op, pool, globals, functions) {
    if name == OP_JUMP_IF_TRUE {
        let v = vPopS(S)
        if vTruthy(v) {
            S["ip"] = S["ip"] + 1 + arg
            return 1
        }
        vBumpIpS(S)
        return 1
    }
    if name == "jump_if_not_nullish" {
        if len(S["stack"]) == 0 {
            throw "vm stack underflow"
        }
        let v = S["stack"][len(S["stack"]) - 1]
        if v != null && v != undefined {
            S["ip"] = S["ip"] + 1 + arg
            return 1
        }
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)

write(
    "vm_ops_jump_result.kab",
    """\
import "self_host/vm_s_stack"

pub fn runOpJumpResult(S, name, arg, op, pool, globals, functions) {
    if name == "jump_if_result_err" {
        let v = vPopS(S)
        if typeof(v) == "object" && v != null && v["vmResult"] == "err" {
            vPushS(S, v["v"])
            S["ip"] = S["ip"] + 1 + arg
            return 1
        }
        if typeof(v) == "object" && v != null && v["vmResult"] == "ok" {
            vPushS(S, v["v"])
        } else {
            vPushS(S, v)
        }
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)

write(
    "vm_ops_jump_a.kab",
    """\
import "self_host/vm_ops_jump_basic"
import "self_host/vm_ops_jump_true"

pub fn runOpJumpA(S, name, arg, op, pool, globals, functions) {
    let st = runOpJumpBasic(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpJumpTrue(S, name, arg, op, pool, globals, functions)
}
""",
)
write(
    "vm_ops_jump_b.kab",
    """\
import "self_host/vm_ops_jump_result"

pub fn runOpJumpB(S, name, arg, op, pool, globals, functions) {
    return runOpJumpResult(S, name, arg, op, pool, globals, functions)
}
""",
)

# ---- CTRL ----
write(
    "vm_ops_ctrl_throw.kab",
    """\
import "self_host/emit_defs"
import "self_host/vm_s_stack"
import "self_host/vm_s_try"

pub fn runOpCtrlThrow(S, name, arg, op, pool, globals, functions) {
    if name == OP_THROW {
        let reason = null
        if len(S["stack"]) > 0 {
            reason = vPopS(S)
        }
        let region = vFindTry(S["tryRegions"], S["ip"])
        if region != null {
            let el = region["errLocal"]
            while len(S["locals"]) <= el {
                S["locals"] = push(S["locals"], undefined)
            }
            S["locals"][el] = reason
            vPushS(S, reason)
            S["ip"] = region["catchStart"]
            return 1
        }
        throw reason
    }
    return 0
}
""",
)

write(
    "vm_ops_ctrl_return.kab",
    """\
import "self_host/emit_defs"

pub fn runOpCtrlReturn(S, name, arg, op, pool, globals, functions) {
    if name == OP_RETURN {
        if len(S["stack"]) > 0 {
            S["result"] = S["stack"][len(S["stack"]) - 1]
        } else {
            S["result"] = null
        }
        S["done"] = true
        return 2
    }
    if name == OP_HALT {
        if len(S["stack"]) > 0 {
            S["result"] = S["stack"][len(S["stack"]) - 1]
        }
        S["done"] = true
        return 2
    }
    return 0
}
""",
)

write(
    "vm_ops_ctrl_a.kab",
    """\
import "self_host/vm_ops_ctrl_throw"
import "self_host/vm_ops_ctrl_return"

pub fn runOpCtrlA(S, name, arg, op, pool, globals, functions) {
    let st = runOpCtrlThrow(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpCtrlReturn(S, name, arg, op, pool, globals, functions)
}
""",
)

print("done")
