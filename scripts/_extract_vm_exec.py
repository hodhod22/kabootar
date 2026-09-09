#!/usr/bin/env python3
"""Extract call/ops/fn/mod glue from vm_run_body via S hooks (break runFn↔runOps cycle)."""
from pathlib import Path

SH = Path("self_host")


def w(name: str, text: str) -> None:
    p = SH / name
    p.write_text(text.lstrip("\n"), encoding="utf-8", newline="\n")
    print("wrote", p)


w(
    "vm_run_call_apply.kab",
    """
/// Apply classified callee via S hooks (runFnBody / runMethodBody).
pub fn vApplyCalleeS(S, c, ordered, pool, globals, functions) {
    let kind = c["kind"]
    if kind == "fn" {
        return S["runFnBody"](functions[c["idx"]], pool, globals, functions, ordered, null)
    }
    if kind == "arrow" {
        return S["runFnBody"](S["arrows"][c["idx"]], pool, globals, functions, ordered, c["caps"])
    }
    if kind == "method" {
        return S["runMethodBody"](c["meth"], pool, globals, functions, c["recv"], ordered)
    }
    return vCallHost(c["callee"], ordered)
}
""",
)

# call_apply needs vCallHost — add import
w(
    "vm_run_call_apply.kab",
    """
import "self_host/vm_s_host"

/// Apply classified callee via S hooks (runFnBody / runMethodBody).
pub fn vApplyCalleeS(S, c, ordered, pool, globals, functions) {
    let kind = c["kind"]
    if kind == "fn" {
        return S["runFnBody"](functions[c["idx"]], pool, globals, functions, ordered, null)
    }
    if kind == "arrow" {
        return S["runFnBody"](S["arrows"][c["idx"]], pool, globals, functions, ordered, c["caps"])
    }
    if kind == "method" {
        return S["runMethodBody"](c["meth"], pool, globals, functions, c["recv"], ordered)
    }
    return vCallHost(c["callee"], ordered)
}
""",
)

w(
    "vm_run_call_cfa.kab",
    """
import "self_host/vm_s_pop"
import "self_host/vm_s_push"
import "self_host/vm_s_ip"
import "self_host/vm_run_args_arr"
import "self_host/vm_run_callee"
import "self_host/vm_run_call_apply"

/// Handle call_from_array. Returns 0=miss, 1=ok.
pub fn runOpCallFromArrayS(S, name, pool, globals, functions) {
    if name != "call_from_array" {
        return 0
    }
    let argsArr = vPopS(S)
    let callee = vPopS(S)
    let ordered = vArrayToOrdered(argsArr)
    let c = vClassifyCallee(callee)
    vPushS(S, vApplyCalleeS(S, c, ordered, pool, globals, functions))
    vBumpIpS(S)
    return 1
}
""",
)

w(
    "vm_run_call_n.kab",
    """
import "self_host/vm_s_pop"
import "self_host/vm_s_push"
import "self_host/vm_s_ip"
import "self_host/vm_run_args_pop"
import "self_host/vm_run_callee"
import "self_host/vm_run_call_apply"

/// Handle call. Returns 0=miss, 1=ok, 2=break.
pub fn runOpCallNS(S, name, arg, pool, globals, functions) {
    if name != "call" {
        return 0
    }
    let callee = vPopS(S)
    let ordered = vPopOrderedArgs(S, arg)
    let c = vClassifyCallee(callee)
    let kind = c["kind"]
    if kind == "bad" {
        S["unsupported"] = true
        return 2
    }
    vPushS(S, vApplyCalleeS(S, c, ordered, pool, globals, functions))
    vBumpIpS(S)
    return 1
}
""",
)

w(
    "vm_run_call.kab",
    """
import "self_host/vm_run_call_cfa"
import "self_host/vm_run_call_n"

pub fn runOpCallS(S, name, arg, op, pool, globals, functions) {
    let st = runOpCallFromArrayS(S, name, pool, globals, functions)
    if st != 0 {
        return st
    }
    return runOpCallNS(S, name, arg, pool, globals, functions)
}
""",
)

w(
    "vm_run_new_run.kab",
    """
import "self_host/vm_run_new_inst"

/// Allocate + run init via S["runMethodBody"].
pub fn vNewInstanceS(S, classIdx, argc, pool, globals, functions) {
    let info = vAllocInstance(S, classIdx, argc)
    let inst = info["inst"]
    let init = info["init"]
    if init != null {
        S["lastThis"] = inst
        S["runMethodBody"](init, pool, globals, functions, inst, info["ordered"])
        if S["lastThis"] != null {
            inst = S["lastThis"]
        }
    } else {
        if argc > 0 {
            throw "vm class has no init: " + info["sym"]
        }
    }
    return inst
}
""",
)

w(
    "vm_run_ops_step.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_ip"
import "self_host/vm_run_dispatch_plain"
import "self_host/vm_run_call"
import "self_host/vm_run_new_run"

/// One op step. Returns 0=continue-loop-as-ok, 1=continue, 2=break. Throws on unsupported.
pub fn runOpsStepS(S, ops, pool, globals, functions) {
    let op = ops[S["ip"]]
    let name = op["op"]
    let arg = op["arg"]
    let st = runOpPlainS(S, name, arg, op, pool, globals, functions)
    if st == 0 {
        if name == "new_instance" {
            let argc = 0
            if op["arg2"] != undefined {
                argc = op["arg2"]
            }
            vPushS(S, vNewInstanceS(S, arg, argc, pool, globals, functions))
            vBumpIpS(S)
            st = 1
        }
    }
    if st == 0 {
        st = runOpCallS(S, name, arg, op, pool, globals, functions)
    }
    if st == 2 {
        return 2
    }
    if st == 0 {
        throw "vm unsupported opcode: " + name
    }
    return 1
}
""",
)

w(
    "vm_run_ops_loop.kab",
    """
import "self_host/vm_run_ops_step"

pub fn runOpsS(S, ops, pool, globals, functions) {
    while S["ip"] < len(ops) && S["done"] == false && S["unsupported"] == false {
        let st = runOpsStepS(S, ops, pool, globals, functions)
        if st == 2 {
            break
        }
    }
}
""",
)

w(
    "vm_run_fn_body.kab",
    """
import "self_host/vm_run_save_fn"
import "self_host/vm_run_restore_fn"
import "self_host/vm_run_prep_fn"
import "self_host/vm_run_take_result"

pub fn runFnBodyS(S, fnDef, pool, globals, functions, callArgs, caps) {
    let old = vSaveFnFrame(S)
    vPrepareFnLocals(S, fnDef, callArgs, caps)
    S["runOps"](fnDef["ops"], pool, globals, functions)
    let out = vTakeResult(S)
    let bad = S["unsupported"]
    vRestoreFnFrame(S, old)
    if bad {
        throw "vm unsupported opcode in fn"
    }
    return out
}
""",
)

w(
    "vm_run_meth_body.kab",
    """
import "self_host/vm_run_this_frame"

pub fn runMethodBodyS(S, meth, pool, globals, functions, recv, callArgs) {
    vPushThis(S, recv)
    let out = null
    let err = null
    try {
        out = S["runFnBody"](meth, pool, globals, functions, callArgs, null)
    } catch (e) {
        err = e
    }
    vPopThis(S)
    if err != null {
        throw err
    }
    return out
}
""",
)

w(
    "vm_run_mod_enter_a.kab",
    """
import "self_host/vm_run_norm_bc"
import "self_host/vm_run_save_mod"
import "self_host/vm_run_alloc_globals"
import "self_host/vm_run_bind_imports"

pub fn vModEnterA(S, bc) {
    if bc == null || bc == undefined {
        throw "vm: null module"
    }
    let n = vNormBc(bc)
    let old = vSaveModuleFrame(S)
    vAllocGlobals(S, len(n["globals"]))
    vBindImports(S, n["globals"], n["imports"])
    return { "n": n, "old": old }
}
""",
)

w(
    "vm_run_mod_enter_b.kab",
    """
import "self_host/vm_run_bind_fns"
import "self_host/vm_run_alloc_locals"
import "self_host/vm_run_reset"
import "self_host/vm_run_fill_hosts"

pub fn vModEnterB(S, n) {
    S["classes"] = n["classes"]
    S["arrows"] = n["arrows"]
    S["tryRegions"] = n["tryRegions"]
    S["localNames"] = n["localNames"]
    vBindFns(S, n["globals"], n["functions"])
    vAllocLocals(S, len(n["localNames"]))
    vResetExec(S)
    vFillHostGlobals(S, n["globals"])
}
""",
)

w(
    "vm_run_mod_enter.kab",
    """
import "self_host/vm_run_mod_enter_a"
import "self_host/vm_run_mod_enter_b"

pub fn vModEnter(S, bc) {
    let a = vModEnterA(S, bc)
    let n = a["n"]
    vModEnterB(S, n)
    return {
        "old": a["old"],
        "pool": n["pool"],
        "globals": n["globals"],
        "functions": n["functions"],
        "ops": n["ops"]
    }
}
""",
)

w(
    "vm_run_mod_leave.kab",
    """
import "self_host/vm_run_take_result"
import "self_host/vm_run_restore_mod"

pub fn vModLeave(S, old) {
    let out = vTakeResult(S)
    let unsupported = S["unsupported"]
    vRestoreModuleFrame(S, old)
    if unsupported {
        throw "vm unsupported opcode in main"
    }
    return out
}
""",
)

# Thin body
w(
    "vm_run_body.kab",
    """
// H6e/P6b: Kabootar bytecode VM subset body — densifying via session object S.
// Exec glue in vm_run_{call,ops,fn,meth,mod}_*; hooks on S break runFn↔runOps cycles.
// Skip-listed until this leaf self-host-compiles ≤10s.
import "self_host/vm_run_fn_body"
import "self_host/vm_run_meth_body"
import "self_host/vm_run_ops_loop"
import "self_host/vm_run_mod_enter"
import "self_host/vm_run_mod_leave"

let S = {
    "stack": [],
    "locals": [],
    "globals": [],
    "classes": [],
    "arrows": [],
    "tryRegions": [],
    "localNames": [],
    "thisStack": [],
    "lastThis": null,
    "nextInstId": 1,
    "ip": 0,
    "done": false,
    "result": null,
    "unsupported": false
}

fn runOps(ops, pool, globals, functions) {
    runOpsS(S, ops, pool, globals, functions)
}

fn runFnBody(fnDef, pool, globals, functions, callArgs, caps) {
    return runFnBodyS(S, fnDef, pool, globals, functions, callArgs, caps)
}

fn runMethodBody(meth, pool, globals, functions, recv, callArgs) {
    return runMethodBodyS(S, meth, pool, globals, functions, recv, callArgs)
}

S["runOps"] = runOps
S["runFnBody"] = runFnBody
S["runMethodBody"] = runMethodBody

pub fn runModuleImplBody(bc) {
    let ctx = vModEnter(S, bc)
    S["runOps"](ctx["ops"], ctx["pool"], ctx["globals"], ctx["functions"])
    return vModLeave(S, ctx["old"])
}
""",
)

print("done")
