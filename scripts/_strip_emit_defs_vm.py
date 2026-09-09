#!/usr/bin/env python3
"""Strip emit_defs from vm_ops_* leaves; use opcode string literals (P6b ≤10s)."""
from pathlib import Path
import re

SH = Path("self_host")

# From emit_defs.kab
OPS = {
    "OP_CONST": "const",
    "OP_POP": "pop",
    "OP_LOAD_GLOBAL": "load_global",
    "OP_STORE_GLOBAL": "store_global",
    "OP_LOAD_LOCAL": "load_local",
    "OP_STORE_LOCAL": "store_local",
    "OP_TAKE_LOCAL": "take_local",
    "OP_TAKE_GLOBAL": "take_global",
    "OP_ARRAY_PUSH_LOCAL": "array_push_local",
    "OP_ARRAY_PUSH_GLOBAL": "array_push_global",
    "OP_ARRAY_POP_LOCAL": "array_pop_local",
    "OP_ARRAY_POP_GLOBAL": "array_pop_global",
    "OP_ACC_ADD_LOCAL": "acc_add_local",
    "OP_ACC_ADD_GLOBAL": "acc_add_global",
    "OP_LEN_LOCAL": "len_local",
    "OP_LEN_GLOBAL": "len_global",
    "OP_INDEX_GET_LOCAL": "index_get_local",
    "OP_INDEX_GET_GLOBAL": "index_get_global",
    "OP_ADD": "add",
    "OP_SUB": "sub",
    "OP_MUL": "mul",
    "OP_DIV": "div",
    "OP_MOD": "mod",
    "OP_POW": "pow",
    "OP_NEG": "neg",
    "OP_BIT_AND": "bit_and",
    "OP_BIT_OR": "bit_or",
    "OP_BIT_XOR": "bit_xor",
    "OP_SHL": "shl",
    "OP_SHR": "shr",
    "OP_USHR": "ushr",
    "OP_BIT_NOT": "bit_not",
    "OP_NOT": "not",
    "OP_AND": "and",
    "OP_OR": "or",
    "OP_EQ": "eq",
    "OP_NE": "ne",
    "OP_LT": "lt",
    "OP_GT": "gt",
    "OP_LE": "le",
    "OP_GE": "ge",
    "OP_MAKE_OBJECT": "make_object",
    "OP_MAKE_ARRAY": "make_array",
    "OP_INDEX_GET": "index_get",
    "OP_INDEX_SET": "index_set",
    "OP_GET_MEMBER": "get_member",
    "OP_SET_MEMBER": "set_member",
    "OP_JUMP": "jump",
    "OP_JUMP_IF_FALSE": "jump_if_false",
    "OP_JUMP_IF_TRUE": "jump_if_true",
    "OP_CALL": "call",
    "OP_NEW_INSTANCE": "new_instance",
    "OP_RETURN": "return",
    "OP_THROW": "throw",
    "OP_JUMP_IF_RESULT_ERR": "jump_if_result_err",
    "OP_MAKE_ARROW": "make_arrow_fn",
    "OP_ITER_STEP": "iterator_step_in_place",
    "OP_MAKE_OK": "make_ok",
    "OP_MAKE_ERR": "make_err",
    "OP_MAKE_SOME": "make_some",
    "OP_MAKE_NONE": "make_none",
    "OP_HALT": "halt",
}

# Longer names first to avoid partial replaces
OP_NAMES = sorted(OPS.keys(), key=len, reverse=True)


def strip_file(path: Path) -> bool:
    text = path.read_text(encoding="utf-8")
    orig = text
    if 'import "self_host/emit_defs"' not in text:
        return False
    # Only strip leaf op files that compare name == OP_*
    text = text.replace('import "self_host/emit_defs"\n', "")
    text = text.replace('import "self_host/emit_defs"\r\n', "")
    for name in OP_NAMES:
        lit = OPS[name]
        text = re.sub(rf"\b{name}\b", f'"{lit}"', text)
    if text != orig:
        path.write_text(text, encoding="utf-8", newline="\n")
        print("stripped", path.name)
        return True
    return False


n = 0
for p in sorted(SH.glob("vm_ops_*.kab")):
    # Keep facades (usually no emit_defs anyway)
    if strip_file(p):
        n += 1
print(f"done strip {n} files")

# Split remaining 2-op OVER candidates into 1-op where still combined
def w(name, body):
    (SH / name).write_text(body.lstrip("\n"), encoding="utf-8", newline="\n")
    print("wrote", name)


# cmp logic → and / or
w(
    "vm_ops_cmp_and.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"
import "self_host/vm_s_truthy"

pub fn runOpCmpAnd(S, name, arg, op, pool, globals, functions) {
    if name == "and" {
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
    return 0
}
""",
)
w(
    "vm_ops_cmp_or.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"
import "self_host/vm_s_truthy"

pub fn runOpCmpOr(S, name, arg, op, pool, globals, functions) {
    if name == "or" {
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
w(
    "vm_ops_cmp_logic.kab",
    """
import "self_host/vm_ops_cmp_and"
import "self_host/vm_ops_cmp_or"

pub fn runOpCmpLogic(S, name, arg, op, pool, globals, functions) {
    let st = runOpCmpAnd(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpCmpOr(S, name, arg, op, pool, globals, functions)
}
""",
)

# cmp eq/ne, ord pairs
w(
    "vm_ops_cmp_eq_only.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpCmpEqOnly(S, name, arg, op, pool, globals, functions) {
    if name == "eq" {
        let b = vPopS(S)
        let a = vPopS(S)
        vPushS(S, a == b)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_cmp_ne.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpCmpNe(S, name, arg, op, pool, globals, functions) {
    if name == "ne" {
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
w(
    "vm_ops_cmp_eq.kab",
    """
import "self_host/vm_ops_cmp_eq_only"
import "self_host/vm_ops_cmp_ne"

pub fn runOpCmpEq(S, name, arg, op, pool, globals, functions) {
    let st = runOpCmpEqOnly(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpCmpNe(S, name, arg, op, pool, globals, functions)
}
""",
)

w(
    "vm_ops_cmp_lt.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpCmpLt(S, name, arg, op, pool, globals, functions) {
    if name == "lt" {
        let b = vPopS(S)
        let a = vPopS(S)
        vPushS(S, a < b)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_cmp_gt.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpCmpGt(S, name, arg, op, pool, globals, functions) {
    if name == "gt" {
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
w(
    "vm_ops_cmp_ord.kab",
    """
import "self_host/vm_ops_cmp_lt"
import "self_host/vm_ops_cmp_gt"

pub fn runOpCmpOrd(S, name, arg, op, pool, globals, functions) {
    let st = runOpCmpLt(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpCmpGt(S, name, arg, op, pool, globals, functions)
}
""",
)

w(
    "vm_ops_cmp_le.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpCmpLe(S, name, arg, op, pool, globals, functions) {
    if name == "le" {
        let b = vPopS(S)
        let a = vPopS(S)
        vPushS(S, a <= b)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_cmp_ge.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpCmpGe(S, name, arg, op, pool, globals, functions) {
    if name == "ge" {
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
w(
    "vm_ops_cmp_ord2.kab",
    """
import "self_host/vm_ops_cmp_le"
import "self_host/vm_ops_cmp_ge"

pub fn runOpCmpOrd2(S, name, arg, op, pool, globals, functions) {
    let st = runOpCmpLe(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpCmpGe(S, name, arg, op, pool, globals, functions)
}
""",
)

# not / get_member split
w(
    "vm_ops_cmp_not.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"
import "self_host/vm_s_truthy"

pub fn runOpCmpNot(S, name, arg, op, pool, globals, functions) {
    if name == "not" {
        let v = vPopS(S)
        vPushS(S, !vTruthy(v))
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_cmp_member.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"
import "self_host/vm_s_member_get"
import "self_host/vm_s_pool_key"

pub fn runOpCmpMember(S, name, arg, op, pool, globals, functions) {
    if name == "get_member" {
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
w(
    "vm_ops_cmp_not_member.kab",
    """
import "self_host/vm_ops_cmp_not"
import "self_host/vm_ops_cmp_member"

pub fn runOpCmpNotMember(S, name, arg, op, pool, globals, functions) {
    let st = runOpCmpNot(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpCmpMember(S, name, arg, op, pool, globals, functions)
}
""",
)

# arith 2-op leftovers
w(
    "vm_ops_arith_mul.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpArithMul(S, name, arg, op, pool, globals, functions) {
    if name == "mul" {
        let b = vPopS(S)
        let a = vPopS(S)
        vPushS(S, a * b)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_arith_div.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpArithDiv(S, name, arg, op, pool, globals, functions) {
    if name == "div" {
        let b = vPopS(S)
        let a = vPopS(S)
        vPushS(S, a / b)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_arith_mul_div.kab",
    """
import "self_host/vm_ops_arith_mul"
import "self_host/vm_ops_arith_div"

pub fn runOpArithMulDiv(S, name, arg, op, pool, globals, functions) {
    let st = runOpArithMul(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpArithDiv(S, name, arg, op, pool, globals, functions)
}
""",
)

w(
    "vm_ops_arith_mod.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpArithMod(S, name, arg, op, pool, globals, functions) {
    if name == "mod" {
        let b = vPopS(S)
        let a = vPopS(S)
        vPushS(S, a % b)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_arith_pow.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpArithPow(S, name, arg, op, pool, globals, functions) {
    if name == "pow" {
        let b = vPopS(S)
        let a = vPopS(S)
        vPushS(S, a ** b)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_arith_mod_pow.kab",
    """
import "self_host/vm_ops_arith_mod"
import "self_host/vm_ops_arith_pow"

pub fn runOpArithModPow(S, name, arg, op, pool, globals, functions) {
    let st = runOpArithMod(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpArithPow(S, name, arg, op, pool, globals, functions)
}
""",
)

w(
    "vm_ops_arith_bit_and.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpArithBitAnd(S, name, arg, op, pool, globals, functions) {
    if name == "bit_and" {
        let b = vPopS(S)
        let a = vPopS(S)
        vPushS(S, a & b)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_arith_bit_or.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpArithBitOr(S, name, arg, op, pool, globals, functions) {
    if name == "bit_or" {
        let b = vPopS(S)
        let a = vPopS(S)
        vPushS(S, a | b)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_arith_bit_and_or.kab",
    """
import "self_host/vm_ops_arith_bit_and"
import "self_host/vm_ops_arith_bit_or"

pub fn runOpArithBitAndOr(S, name, arg, op, pool, globals, functions) {
    let st = runOpArithBitAnd(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpArithBitOr(S, name, arg, op, pool, globals, functions)
}
""",
)

w(
    "vm_ops_arith_bit_xor.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpArithBitXor(S, name, arg, op, pool, globals, functions) {
    if name == "bit_xor" {
        let b = vPopS(S)
        let a = vPopS(S)
        vPushS(S, a ^ b)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_arith_bit_not.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpArithBitNot(S, name, arg, op, pool, globals, functions) {
    if name == "bit_not" {
        let v = vPopS(S)
        vPushS(S, ~v)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_arith_bit_xor_not.kab",
    """
import "self_host/vm_ops_arith_bit_xor"
import "self_host/vm_ops_arith_bit_not"

pub fn runOpArithBitXorNot(S, name, arg, op, pool, globals, functions) {
    let st = runOpArithBitXor(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpArithBitNot(S, name, arg, op, pool, globals, functions)
}
""",
)

w(
    "vm_ops_arith_shl.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpArithShl(S, name, arg, op, pool, globals, functions) {
    if name == "shl" {
        let b = vPopS(S)
        let a = vPopS(S)
        vPushS(S, a << b)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_arith_shr.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpArithShr(S, name, arg, op, pool, globals, functions) {
    if name == "shr" {
        let b = vPopS(S)
        let a = vPopS(S)
        vPushS(S, a >> b)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_arith_shift.kab",
    """
import "self_host/vm_ops_arith_shl"
import "self_host/vm_ops_arith_shr"

pub fn runOpArithShift(S, name, arg, op, pool, globals, functions) {
    let st = runOpArithShl(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpArithShr(S, name, arg, op, pool, globals, functions)
}
""",
)

# ctrl throw: extract catch path
w(
    "vm_s_catch.kab",
    """
import "self_host/vm_s_push"

pub fn vCatchThrow(S, reason) {
    let region = vFindTry(S["tryRegions"], S["ip"])
    if region == null {
        return 0
    }
    let el = region["errLocal"]
    while len(S["locals"]) <= el {
        S["locals"] = push(S["locals"], undefined)
    }
    S["locals"][el] = reason
    vPushS(S, reason)
    S["ip"] = region["catchStart"]
    return 1
}
""",
)

# Fix: vFindTry needs import
w(
    "vm_s_catch.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_try"

pub fn vCatchThrow(S, reason) {
    let region = vFindTry(S["tryRegions"], S["ip"])
    if region == null {
        return 0
    }
    let el = region["errLocal"]
    while len(S["locals"]) <= el {
        S["locals"] = push(S["locals"], undefined)
    }
    S["locals"][el] = reason
    vPushS(S, reason)
    S["ip"] = region["catchStart"]
    return 1
}
""",
)
w(
    "vm_ops_ctrl_throw.kab",
    """
import "self_host/vm_s_pop"
import "self_host/vm_s_catch"

pub fn runOpCtrlThrow(S, name, arg, op, pool, globals, functions) {
    if name == "throw" {
        let reason = null
        if len(S["stack"]) > 0 {
            reason = vPopS(S)
        }
        if vCatchThrow(S, reason) == 1 {
            return 1
        }
        throw reason
    }
    return 0
}
""",
)

# this_is: split checks
w(
    "vm_s_this_is_obj.kab",
    """
pub fn vIsCurrentThisObj(obj, cur) {
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
    "vm_s_this_is.kab",
    """
import "self_host/vm_s_this_is_obj"

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
    return vIsCurrentThisObj(obj, cur)
}
""",
)

# data helpers that were OVER: split len/concat, stack dup/swap, result ok/err
w(
    "vm_ops_data_len.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpDataLen(S, name, arg, op, pool, globals, functions) {
    if name == "get_length" {
        let v = vPopS(S)
        vPushS(S, len(v))
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_data_concat.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpDataConcat(S, name, arg, op, pool, globals, functions) {
    if name == "concat_array" {
        let right = vPopS(S)
        let left = vPopS(S)
        let out = left
        let i = 0
        while i < len(right) {
            out = push(out, right[i])
            i = i + 1
        }
        vPushS(S, out)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_data_len_concat.kab",
    """
import "self_host/vm_ops_data_len"
import "self_host/vm_ops_data_concat"

pub fn runOpDataLenConcat(S, name, arg, op, pool, globals, functions) {
    let st = runOpDataLen(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpDataConcat(S, name, arg, op, pool, globals, functions)
}
""",
)

# Read current data_stack / result / merge to split if needed - write 1-op versions
w(
    "vm_ops_data_dup.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_ip"

pub fn runOpDataDup(S, name, arg, op, pool, globals, functions) {
    if name == "dup" {
        if len(S["stack"]) == 0 {
            throw "vm stack underflow"
        }
        vPushS(S, S["stack"][len(S["stack"]) - 1])
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_data_swap.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpDataSwap(S, name, arg, op, pool, globals, functions) {
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
w(
    "vm_ops_data_stack.kab",
    """
import "self_host/vm_ops_data_dup"
import "self_host/vm_ops_data_swap"

pub fn runOpDataStack(S, name, arg, op, pool, globals, functions) {
    let st = runOpDataDup(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpDataSwap(S, name, arg, op, pool, globals, functions)
}
""",
)

w(
    "vm_ops_data_ok.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpDataOk(S, name, arg, op, pool, globals, functions) {
    if name == "make_ok" {
        vPushS(S, { "vmResult": "ok", "v": vPopS(S) })
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_data_err.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpDataErr(S, name, arg, op, pool, globals, functions) {
    if name == "make_err" {
        vPushS(S, { "vmResult": "err", "v": vPopS(S) })
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_data_result.kab",
    """
import "self_host/vm_ops_data_ok"
import "self_host/vm_ops_data_err"

pub fn runOpDataResult(S, name, arg, op, pool, globals, functions) {
    let st = runOpDataOk(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpDataErr(S, name, arg, op, pool, globals, functions)
}
""",
)

# mem 2-op splits that were OVER
w(
    "vm_ops_mem_const.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_ip"
import "self_host/vm_s_pool_val"

pub fn runOpMemConst(S, name, arg, op, pool, globals, functions) {
    if name == "const" {
        vPushS(S, poolVal(pool, arg))
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_mem_pop.kab",
    """
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpMemPop(S, name, arg, op, pool, globals, functions) {
    if name == "pop" {
        vPopS(S)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_mem_const_pop.kab",
    """
import "self_host/vm_ops_mem_const"
import "self_host/vm_ops_mem_pop"

pub fn runOpMemConstPop(S, name, arg, op, pool, globals, functions) {
    let st = runOpMemConst(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpMemPop(S, name, arg, op, pool, globals, functions)
}
""",
)

w(
    "vm_ops_mem_load_local.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_ip"

pub fn runOpMemLoadLocal(S, name, arg, op, pool, globals, functions) {
    if name == "load_local" {
        vPushS(S, S["locals"][arg])
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_mem_store_local.kab",
    """
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpMemStoreLocal(S, name, arg, op, pool, globals, functions) {
    if name == "store_local" {
        let v = vPopS(S)
        S["locals"][arg] = v
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_mem_local.kab",
    """
import "self_host/vm_ops_mem_load_local"
import "self_host/vm_ops_mem_store_local"

pub fn runOpMemLocal(S, name, arg, op, pool, globals, functions) {
    let st = runOpMemLoadLocal(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpMemStoreLocal(S, name, arg, op, pool, globals, functions)
}
""",
)

# array_pop_bind, acc_add, index_get mem, take — split similarly if needed after measure
w(
    "vm_ops_mem_array_pop_local.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_ip"

pub fn runOpMemArrayPopLocal(S, name, arg, op, pool, globals, functions) {
    if name == "array_pop_local" {
        let arr = S["locals"][arg]
        S["locals"][arg] = pop(arr)
        vPushS(S, null)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_mem_array_pop_global.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_ip"

pub fn runOpMemArrayPopGlobal(S, name, arg, op, pool, globals, functions) {
    if name == "array_pop_global" {
        let arr = S["globals"][arg]
        S["globals"][arg] = pop(arr)
        vPushS(S, null)
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_mem_array_pop_bind.kab",
    """
import "self_host/vm_ops_mem_array_pop_local"
import "self_host/vm_ops_mem_array_pop_global"

pub fn runOpMemArrayPopBind(S, name, arg, op, pool, globals, functions) {
    let st = runOpMemArrayPopLocal(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpMemArrayPopGlobal(S, name, arg, op, pool, globals, functions)
}
""",
)

w(
    "vm_ops_mem_acc_add_local.kab",
    """
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"

pub fn runOpMemAccAddLocal(S, name, arg, op, pool, globals, functions) {
    if name == "acc_add_local" {
        let rhs = vPopS(S)
        S["locals"][arg] = S["locals"][arg] + rhs
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_mem_acc_add_global.kab",
    """
import "self_host/vm_s_pop"
import "self_host/vm_s_ip"
import "self_host/vm_s_resolve"

pub fn runOpMemAccAddGlobal(S, name, arg, op, pool, globals, functions) {
    if name == "acc_add_global" {
        let rhs = vPopS(S)
        let cur = vResolveGlobalS(S, globals, arg)
        S["globals"][arg] = cur + rhs
        vBumpIpS(S)
        return 1
    }
    return 0
}
""",
)
w(
    "vm_ops_mem_acc_add.kab",
    """
import "self_host/vm_ops_mem_acc_add_local"
import "self_host/vm_ops_mem_acc_add_global"

pub fn runOpMemAccAdd(S, name, arg, op, pool, globals, functions) {
    let st = runOpMemAccAddLocal(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpMemAccAddGlobal(S, name, arg, op, pool, globals, functions)
}
""",
)

print("done splits")
