#!/usr/bin/env python3
"""Split OVER VM shards into smaller leaves (P6b ≤10s)."""
from pathlib import Path

SH = Path("self_host")


def w(name: str, text: str) -> None:
    p = SH / name
    p.write_text(text.lstrip("\n"), encoding="utf-8", newline="\n")
    print("wrote", p)


# --- stack ---
w(
    "vm_s_push.kab",
    """
// P6b: push onto session stack.
pub fn vPushS(S, v) {
    S["stack"] = push(S["stack"], v)
}
""",
)
w(
    "vm_s_pop.kab",
    """
// P6b: pop from session stack.
pub fn vPopS(S) {
    let st = S["stack"]
    if len(st) == 0 {
        throw "vm stack underflow"
    }
    let top = st[len(st) - 1]
    S["stack"] = pop(st)
    return top
}
""",
)
w(
    "vm_s_ip.kab",
    """
// P6b: ip helpers on session S.
pub fn vBumpIpS(S) {
    S["ip"] = S["ip"] + 1
}

/// Add delta to S.ip (avoid `S["ip"] = S["ip"] + 1 + arg` — self-host emit rejects that).
pub fn vAddIpS(S, delta) {
    let n = S["ip"] + delta
    S["ip"] = n
}
""",
)
w(
    "vm_s_stack.kab",
    """
// P6b: stack/ip facade — re-exports via import side effects for dependents that only import this.
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"
""",
)

# --- this ---
w(
    "vm_s_this_get.kab",
    """
pub fn vThisS(S) {
    if len(S["thisStack"]) == 0 {
        return undefined
    }
    return S["thisStack"][len(S["thisStack"]) - 1]
}
""",
)
w(
    "vm_s_this_write.kab",
    """
pub fn vWriteThisS(S, updated) {
    if len(S["thisStack"]) == 0 {
        return
    }
    S["thisStack"] = pop(S["thisStack"])
    S["thisStack"] = push(S["thisStack"], updated)
}
""",
)
w(
    "vm_s_this_is.kab",
    """
pub fn vIsCurrentThisS(S, obj) {
    if obj == null || obj == undefined {
        return false
    }
    if len(S["thisStack"]) == 0 {
        return false
    }
    let cur = S["thisStack"][len(S["thisStack"]) - 1]
    if cur == null || cur == undefined {
        return false
    }
    if typeof(obj) != "object" || typeof(cur) != "object" {
        return false
    }
    if obj["vmI"] == undefined || cur["vmI"] == undefined {
        return false
    }
    return obj["vmI"] == cur["vmI"]
}
""",
)
w(
    "vm_s_this.kab",
    """
import "self_host/vm_s_this_get"
import "self_host/vm_s_this_write"
import "self_host/vm_s_this_is"
""",
)

# --- member / index ---
w(
    "vm_s_member_get.kab",
    """
import "self_host/vm_s_find_method"

pub fn vMemberGetS(S, obj, key) {
    if obj == null || obj == undefined {
        return undefined
    }
    if typeof(obj) == "object" {
        if obj["vmC"] != undefined {
            if object_has_own(obj, key) {
                return obj[key]
            }
            let ci = obj["vmC"]
            let meth = vFindMethod(S["classes"][ci], key)
            if meth != null {
                return { "vmM": meth, "vmR": obj }
            }
            return undefined
        }
    }
    return obj[key]
}
""",
)
w(
    "vm_s_member_set.kab",
    """
pub fn vMemberSet(obj, key, val) {
    if obj == null || obj == undefined {
        throw "vm cannot set member on nullish"
    }
    obj[key] = val
    return obj
}
""",
)
w(
    "vm_s_index_get.kab",
    """
pub fn vIndexGet(container, idx) {
    if container == null || container == undefined {
        return undefined
    }
    return container[idx]
}
""",
)
w(
    "vm_s_index_set.kab",
    """
pub fn vIndexSet(container, idx, val) {
    if container == null || container == undefined {
        throw "vm cannot index nullish"
    }
    container[idx] = val
    return container
}
""",
)
w(
    "vm_s_member.kab",
    """
import "self_host/vm_s_member_get"
import "self_host/vm_s_member_set"
import "self_host/vm_s_index_get"
import "self_host/vm_s_index_set"
""",
)

# --- data make ---
w(
    "vm_ops_data_make_object.kab",
    """
import "self_host/emit_defs"
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpDataMakeObject(S, name, arg, op, pool, globals, functions) {
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
    return 0
}
""",
)
w(
    "vm_ops_data_make_array.kab",
    """
import "self_host/emit_defs"
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpDataMakeArray(S, name, arg, op, pool, globals, functions) {
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
w(
    "vm_ops_data_make.kab",
    """
import "self_host/vm_ops_data_make_object"
import "self_host/vm_ops_data_make_array"

pub fn runOpDataMake(S, name, arg, op, pool, globals, functions) {
    let st = runOpDataMakeObject(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpDataMakeArray(S, name, arg, op, pool, globals, functions)
}
""",
)

# --- data index ---
w(
    "vm_ops_data_index_get.kab",
    """
import "self_host/emit_defs"
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"
import "self_host/vm_s_index_get"

pub fn runOpDataIndexGet(S, name, arg, op, pool, globals, functions) {
    if name == OP_INDEX_GET {
        let idx = vPopS(S)
        let container = vPopS(S)
        vPushS(S, vIndexGet(container, idx))
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_data_index_set.kab",
    """
import "self_host/emit_defs"
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"
import "self_host/vm_s_index_set"
import "self_host/vm_s_this_is"
import "self_host/vm_s_this_write"

pub fn runOpDataIndexSet(S, name, arg, op, pool, globals, functions) {
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
w(
    "vm_ops_data_index.kab",
    """
import "self_host/vm_ops_data_index_get"
import "self_host/vm_ops_data_index_set"

pub fn runOpDataIndex(S, name, arg, op, pool, globals, functions) {
    let st = runOpDataIndexGet(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpDataIndexSet(S, name, arg, op, pool, globals, functions)
}
""",
)

# --- data option ---
w(
    "vm_ops_data_option_some.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpDataOptionSome(S, name, arg, op, pool, globals, functions) {
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
    return 0
}
""",
)
w(
    "vm_ops_data_option_arrow.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_ip"
import "self_host/vm_s_caps"

pub fn runOpDataOptionArrow(S, name, arg, op, pool, globals, functions) {
    if name == "make_arrow_fn" {
        vPushS(S, { "vmArrow": arg, "caps": vSnapCapsS(S) })
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_data_option_iter.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpDataOptionIter(S, name, arg, op, pool, globals, functions) {
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
w(
    "vm_ops_data_option.kab",
    """
import "self_host/vm_ops_data_option_some"
import "self_host/vm_ops_data_option_arrow"
import "self_host/vm_ops_data_option_iter"

pub fn runOpDataOption(S, name, arg, op, pool, globals, functions) {
    let st = runOpDataOptionSome(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    st = runOpDataOptionArrow(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpDataOptionIter(S, name, arg, op, pool, globals, functions)
}
""",
)

# --- jump true ---
w(
    "vm_ops_jump_if_true.kab",
    """
import "self_host/emit_defs"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"
import "self_host/vm_s_truthy"

pub fn runOpJumpIfTrue(S, name, arg, op, pool, globals, functions) {
    if name == OP_JUMP_IF_TRUE {
        let v = vPopS(S)
        if vTruthy(v) {
            vAddIpS(S, 1 + arg)
            return 1
        }
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_jump_nullish.kab",
    """
import "self_host/vm_s_ip"

pub fn runOpJumpNullish(S, name, arg, op, pool, globals, functions) {
    if name == "jump_if_not_nullish" {
        if len(S["stack"]) == 0 {
            throw "vm stack underflow"
        }
        let v = S["stack"][len(S["stack"]) - 1]
        if v != null && v != undefined {
            vAddIpS(S, 1 + arg)
            return 1
        }
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_jump_true.kab",
    """
import "self_host/vm_ops_jump_if_true"
import "self_host/vm_ops_jump_nullish"

pub fn runOpJumpTrue(S, name, arg, op, pool, globals, functions) {
    let st = runOpJumpIfTrue(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpJumpNullish(S, name, arg, op, pool, globals, functions)
}
""",
)

# --- arith add/sub ---
w(
    "vm_ops_arith_add.kab",
    """
import "self_host/emit_defs"
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpArithAdd(S, name, arg, op, pool, globals, functions) {
    if name == OP_ADD {
        let b = vPopS(S)
        let a = vPopS(S)
        vPushS(S, a + b)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_arith_sub.kab",
    """
import "self_host/emit_defs"
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpArithSub(S, name, arg, op, pool, globals, functions) {
    if name == OP_SUB {
        let b = vPopS(S)
        let a = vPopS(S)
        vPushS(S, a - b)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_arith_add_sub.kab",
    """
import "self_host/vm_ops_arith_add"
import "self_host/vm_ops_arith_sub"

pub fn runOpArithAddSub(S, name, arg, op, pool, globals, functions) {
    let st = runOpArithAdd(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpArithSub(S, name, arg, op, pool, globals, functions)
}
""",
)

# --- mem array push bind ---
w(
    "vm_ops_mem_array_push_local.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpMemArrayPushLocal(S, name, arg, op, pool, globals, functions) {
    if name == "array_push_local" {
        let item = vPopS(S)
        let arr = vPopS(S)
        let next = bytecode_array_push(arr, item)
        S["locals"][arg] = next
        vPushS(S, len(next))
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_mem_array_push_global.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpMemArrayPushGlobal(S, name, arg, op, pool, globals, functions) {
    if name == "array_push_global" {
        let item = vPopS(S)
        let arr = vPopS(S)
        let next = bytecode_array_push(arr, item)
        S["globals"][arg] = next
        vPushS(S, len(next))
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_mem_array_push_bind.kab",
    """
import "self_host/vm_ops_mem_array_push_local"
import "self_host/vm_ops_mem_array_push_global"

pub fn runOpMemArrayPushBind(S, name, arg, op, pool, globals, functions) {
    let st = runOpMemArrayPushLocal(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpMemArrayPushGlobal(S, name, arg, op, pool, globals, functions)
}
""",
)

# --- jump basic: already 2 ops, split ---
w(
    "vm_ops_jump_goto.kab",
    """
import "self_host/emit_defs"
import "self_host/vm_s_ip"

pub fn runOpJumpGoto(S, name, arg, op, pool, globals, functions) {
    if name == OP_JUMP {
        vAddIpS(S, 1 + arg)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_jump_if_false.kab",
    """
import "self_host/emit_defs"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"
import "self_host/vm_s_truthy"

pub fn runOpJumpIfFalse(S, name, arg, op, pool, globals, functions) {
    if name == OP_JUMP_IF_FALSE {
        let v = vPopS(S)
        if !vTruthy(v) {
            vAddIpS(S, 1 + arg)
            return 1
        }
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_jump_basic.kab",
    """
import "self_host/vm_ops_jump_goto"
import "self_host/vm_ops_jump_if_false"

pub fn runOpJumpBasic(S, name, arg, op, pool, globals, functions) {
    let st = runOpJumpGoto(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpJumpIfFalse(S, name, arg, op, pool, globals, functions)
}
""",
)

# --- ctrl return / throw: use string literals to avoid emit_defs weight ---
w(
    "vm_ops_ctrl_return.kab",
    """
import "self_host/vm_s_ip"

pub fn runOpCtrlReturn(S, name, arg, op, pool, globals, functions) {
    if name == "return" {
        if len(S["stack"]) > 0 {
            let top = S["stack"][len(S["stack"]) - 1]
            S["result"] = top
        } else {
            S["result"] = null
        }
        S["done"] = true
        return 2
    }
    if name == "halt" {
        if len(S["stack"]) > 0 {
            let top = S["stack"][len(S["stack"]) - 1]
            S["result"] = top
        }
        S["done"] = true
        return 2
    }
    return 0
}
""",
)
w(
    "vm_ops_ctrl_return_ret.kab",
    """
pub fn runOpCtrlReturnRet(S, name, arg, op, pool, globals, functions) {
    if name == "return" {
        if len(S["stack"]) > 0 {
            let top = S["stack"][len(S["stack"]) - 1]
            S["result"] = top
        } else {
            S["result"] = null
        }
        S["done"] = true
        return 2
    }
    return 0
}
""",
)
w(
    "vm_ops_ctrl_return_halt.kab",
    """
pub fn runOpCtrlReturnHalt(S, name, arg, op, pool, globals, functions) {
    if name == "halt" {
        if len(S["stack"]) > 0 {
            let top = S["stack"][len(S["stack"]) - 1]
            S["result"] = top
        }
        S["done"] = true
        return 2
    }
    return 0
}
""",
)
w(
    "vm_ops_ctrl_return.kab",
    """
import "self_host/vm_ops_ctrl_return_ret"
import "self_host/vm_ops_ctrl_return_halt"

pub fn runOpCtrlReturn(S, name, arg, op, pool, globals, functions) {
    let st = runOpCtrlReturnRet(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpCtrlReturnHalt(S, name, arg, op, pool, globals, functions)
}
""",
)
w(
    "vm_ops_ctrl_throw.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_try"

pub fn runOpCtrlThrow(S, name, arg, op, pool, globals, functions) {
    if name == "throw" {
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

# Update data_set_member to import leaf this/member
w(
    "vm_ops_data_set_member.kab",
    """
import "self_host/emit_defs"
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"
import "self_host/vm_s_member_set"
import "self_host/vm_s_this_is"
import "self_host/vm_s_this_write"
import "self_host/vm_s_pool"

pub fn runOpDataSetMember(S, name, arg, op, pool, globals, functions) {
    if name == OP_SET_MEMBER {
        let key = poolVal(pool, arg)
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

# cmp not_member uses vMemberGetS
w(
    "vm_ops_cmp_not_member.kab",
    """
import "self_host/emit_defs"
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"
import "self_host/vm_s_truthy"
import "self_host/vm_s_member_get"
import "self_host/vm_s_pool"

pub fn runOpCmpNotMember(S, name, arg, op, pool, globals, functions) {
    if name == OP_NOT {
        let v = vPopS(S)
        vPushS(S, !vTruthy(v))
        vBumpIpS(S)
        return 1
    }
    if name == OP_MEMBER {
        let key = poolVal(pool, arg)
        let obj = vPopS(S)
        vPushS(S, vMemberGetS(S, obj, key))
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)

# resolve uses vThisS
resolve = (SH / "vm_s_resolve.kab").read_text(encoding="utf-8")
if 'import "self_host/vm_s_this"' in resolve:
    resolve = resolve.replace(
        'import "self_host/vm_s_this"', 'import "self_host/vm_s_this_get"'
    )
    (SH / "vm_s_resolve.kab").write_text(resolve, encoding="utf-8", newline="\n")
    print("patched vm_s_resolve.kab")

# jump_result: use leaf imports
w(
    "vm_ops_jump_result.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpJumpResult(S, name, arg, op, pool, globals, functions) {
    if name == "jump_if_result_err" {
        let v = vPopS(S)
        if typeof(v) == "object" && v != null && v["vmResult"] == "err" {
            vPushS(S, v["v"])
            vAddIpS(S, 1 + arg)
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

print("done splits")
