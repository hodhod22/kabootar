#!/usr/bin/env python3
"""Split OVER vm_run_dispatch_* / callee / new_inst into ≤2-import leaves (P6b ≤10s)."""
from pathlib import Path

SH = Path("self_host")


def w(name: str, text: str) -> None:
    p = SH / name
    p.write_text(text.lstrip("\n"), encoding="utf-8", newline="\n")
    print("wrote", p)


def chain2(name: str, fn: str, a_mod: str, a_fn: str, b_mod: str, b_fn: str) -> None:
    w(
        name,
        f"""
// P6b: 2-way dispatch chain (≤10s self-host compile).
import "self_host/{a_mod}"
import "self_host/{b_mod}"

pub fn {fn}(S, name, arg, op, pool, globals, functions) {{
    let st = {a_fn}(S, name, arg, op, pool, globals, functions)
    if st != 0 {{
        return st
    }}
    return {b_fn}(S, name, arg, op, pool, globals, functions)
}}
""",
    )


# --- mem: A-B / C-D / CD-E / AB-(CDE) ---
chain2(
    "vm_run_dispatch_mem_ab.kab",
    "runOpMemAb",
    "vm_ops_mem_a",
    "runOpMemA",
    "vm_ops_mem_b",
    "runOpMemB",
)
chain2(
    "vm_run_dispatch_mem_cd.kab",
    "runOpMemCd",
    "vm_ops_mem_c",
    "runOpMemC",
    "vm_ops_mem_d",
    "runOpMemD",
)
w(
    "vm_run_dispatch_mem_e.kab",
    """
import "self_host/vm_ops_mem_e"

pub fn runOpMemEwrap(S, name, arg, op, pool, globals, functions) {
    return runOpMemE(S, name, arg, op, pool, globals, functions)
}
""",
)
chain2(
    "vm_run_dispatch_mem_cde.kab",
    "runOpMemCde",
    "vm_run_dispatch_mem_cd",
    "runOpMemCd",
    "vm_run_dispatch_mem_e",
    "runOpMemEwrap",
)
chain2(
    "vm_run_dispatch_mem.kab",
    "runOpMemS",
    "vm_run_dispatch_mem_ab",
    "runOpMemAb",
    "vm_run_dispatch_mem_cde",
    "runOpMemCde",
)

# --- arith ---
chain2(
    "vm_run_dispatch_arith_ab.kab",
    "runOpArithAb",
    "vm_ops_arith_a",
    "runOpArithA",
    "vm_ops_arith_b",
    "runOpArithB",
)
chain2(
    "vm_run_dispatch_arith_cd.kab",
    "runOpArithCd",
    "vm_ops_arith_c",
    "runOpArithC",
    "vm_ops_arith_d",
    "runOpArithD",
)
chain2(
    "vm_run_dispatch_arith.kab",
    "runOpArithS",
    "vm_run_dispatch_arith_ab",
    "runOpArithAb",
    "vm_run_dispatch_arith_cd",
    "runOpArithCd",
)

# --- cmp ---
chain2(
    "vm_run_dispatch_cmp_ab.kab",
    "runOpCmpAb",
    "vm_ops_cmp_a",
    "runOpCmpA",
    "vm_ops_cmp_b",
    "runOpCmpB",
)
chain2(
    "vm_run_dispatch_cmp.kab",
    "runOpCmpS",
    "vm_run_dispatch_cmp_ab",
    "runOpCmpAb",
    "vm_ops_cmp_c",
    "runOpCmpC",
)

# --- data ---
chain2(
    "vm_run_dispatch_data_ab.kab",
    "runOpDataAb",
    "vm_ops_data_a",
    "runOpDataA",
    "vm_ops_data_b",
    "runOpDataB",
)
chain2(
    "vm_run_dispatch_data_cd.kab",
    "runOpDataCd",
    "vm_ops_data_c",
    "runOpDataC",
    "vm_ops_data_d",
    "runOpDataD",
)
chain2(
    "vm_run_dispatch_data.kab",
    "runOpDataS",
    "vm_run_dispatch_data_ab",
    "runOpDataAb",
    "vm_run_dispatch_data_cd",
    "runOpDataCd",
)

# --- plain binary tree ---
chain2(
    "vm_run_dispatch_plain_ma.kab",
    "runOpPlainMa",
    "vm_run_dispatch_mem",
    "runOpMemS",
    "vm_run_dispatch_arith",
    "runOpArithS",
)
chain2(
    "vm_run_dispatch_plain_cd.kab",
    "runOpPlainCd",
    "vm_run_dispatch_cmp",
    "runOpCmpS",
    "vm_run_dispatch_data",
    "runOpDataS",
)
chain2(
    "vm_run_dispatch_plain_jc.kab",
    "runOpPlainJc",
    "vm_run_dispatch_jump",
    "runOpJumpS",
    "vm_run_dispatch_ctrl",
    "runOpCtrlS",
)
chain2(
    "vm_run_dispatch_plain_macd.kab",
    "runOpPlainMacd",
    "vm_run_dispatch_plain_ma",
    "runOpPlainMa",
    "vm_run_dispatch_plain_cd",
    "runOpPlainCd",
)
chain2(
    "vm_run_dispatch_plain.kab",
    "runOpPlainS",
    "vm_run_dispatch_plain_macd",
    "runOpPlainMacd",
    "vm_run_dispatch_plain_jc",
    "runOpPlainJc",
)

# --- callee ---
w(
    "vm_run_callee_fn.kab",
    """
import "self_host/vm_s_host"

/// Classify fn|arrow|miss. miss => kind null for next stage.
pub fn vClassifyFnArrow(callee) {
    if callee == null || callee == undefined {
        return { "kind": "bad" }
    }
    let vmFn = vObjGet(callee, "vmFn")
    if vmFn != undefined {
        return { "kind": "fn", "idx": vmFn }
    }
    let vmArrow = vObjGet(callee, "vmArrow")
    if vmArrow != undefined {
        return { "kind": "arrow", "idx": vmArrow, "caps": vObjGet(callee, "caps") }
    }
    return { "kind": null, "callee": callee }
}
""",
)
w(
    "vm_run_callee_meth.kab",
    """
import "self_host/vm_s_host"

/// Classify method|host given non-null callee that is not fn/arrow.
pub fn vClassifyMethHost(callee) {
    let vmM = vObjGet(callee, "vmM")
    if vmM != undefined {
        return { "kind": "method", "meth": vmM, "recv": vObjGet(callee, "vmR") }
    }
    return { "kind": "host", "callee": callee }
}
""",
)
w(
    "vm_run_callee.kab",
    """
import "self_host/vm_run_callee_fn"
import "self_host/vm_run_callee_meth"

/// Classify callee for call ops. kind: fn|arrow|method|host|bad
pub fn vClassifyCallee(callee) {
    let c = vClassifyFnArrow(callee)
    if c["kind"] != null {
        return c
    }
    return vClassifyMethHost(c["callee"])
}
""",
)

# --- new_inst ---
w(
    "vm_run_new_shell.kab",
    """
import "self_host/vm_run_args_pop"

/// Pop ctor args and allocate instance shell.
pub fn vAllocInstShell(S, classIdx, argc) {
    if classIdx < 0 || classIdx >= len(S["classes"]) {
        throw "vm bad class index"
    }
    let classDef = S["classes"][classIdx]
    let ordered = vPopOrderedArgs(S, argc)
    let id = S["nextInstId"]
    S["nextInstId"] = S["nextInstId"] + 1
    let inst = { "vmC": classIdx, "vmI": id }
    return {
        "inst": inst,
        "ordered": ordered,
        "classDef": classDef
    }
}
""",
)
w(
    "vm_run_new_init.kab",
    """
import "self_host/vm_s_find_method"

/// Resolve init/constructor on classDef.
pub fn vResolveInit(classDef) {
    let init = vFindMethod(classDef, "init")
    if init == null {
        init = vFindMethod(classDef, "constructor")
    }
    return init
}
""",
)
w(
    "vm_run_new_inst.kab",
    """
import "self_host/vm_run_new_shell"
import "self_host/vm_run_new_init"

/// Allocate instance shell + resolve init method (no runMethodBody).
pub fn vAllocInstance(S, classIdx, argc) {
    let sh = vAllocInstShell(S, classIdx, argc)
    let init = vResolveInit(sh["classDef"])
    return {
        "inst": sh["inst"],
        "init": init,
        "ordered": sh["ordered"],
        "sym": sh["classDef"]["sym"]
    }
}
""",
)

print("done")
