#!/usr/bin/env python3
"""Split OVER exec leaves + thin body further (P6b ≤10s)."""
from pathlib import Path

SH = Path("self_host")


def w(name: str, text: str) -> None:
    p = SH / name
    p.write_text(text.lstrip("\n"), encoding="utf-8", newline="\n")
    print("wrote", p)


w(
    "vm_run_call_apply_fn.kab",
    """
/// Apply fn|arrow via S["runFnBody"]. Returns null if not fn/arrow.
pub fn vApplyFnArrowS(S, c, ordered, pool, globals, functions) {
    let kind = c["kind"]
    if kind == "fn" {
        return S["runFnBody"](functions[c["idx"]], pool, globals, functions, ordered, null)
    }
    if kind == "arrow" {
        return S["runFnBody"](S["arrows"][c["idx"]], pool, globals, functions, ordered, c["caps"])
    }
    return undefined
}
""",
)

w(
    "vm_run_call_apply_mh.kab",
    """
import "self_host/vm_s_host"

/// Apply method|host|bad via hooks / host call.
pub fn vApplyMethHostS(S, c, ordered, pool, globals, functions) {
    let kind = c["kind"]
    if kind == "method" {
        return S["runMethodBody"](c["meth"], pool, globals, functions, c["recv"], ordered)
    }
    return vCallHost(c["callee"], ordered)
}
""",
)

w(
    "vm_run_call_apply.kab",
    """
import "self_host/vm_run_call_apply_fn"
import "self_host/vm_run_call_apply_mh"

pub fn vApplyCalleeS(S, c, ordered, pool, globals, functions) {
    let kind = c["kind"]
    if kind == "fn" || kind == "arrow" {
        return vApplyFnArrowS(S, c, ordered, pool, globals, functions)
    }
    return vApplyMethHostS(S, c, ordered, pool, globals, functions)
}
""",
)

w(
    "vm_run_new_run_init.kab",
    """
/// Run init on shell if present; update lastThis. Returns inst.
pub fn vRunNewInitS(S, info, argc, pool, globals, functions) {
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
    "vm_run_new_run.kab",
    """
import "self_host/vm_run_new_inst"
import "self_host/vm_run_new_run_init"

pub fn vNewInstanceS(S, classIdx, argc, pool, globals, functions) {
    let info = vAllocInstance(S, classIdx, argc)
    return vRunNewInitS(S, info, argc, pool, globals, functions)
}
""",
)

w(
    "vm_run_ops_plain_new.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_ip"
import "self_host/vm_run_dispatch_plain"
import "self_host/vm_run_new_run"

/// Try plain ops then new_instance. 0=miss, 1=ok.
pub fn runOpsPlainNewS(S, name, arg, op, pool, globals, functions) {
    let st = runOpPlainS(S, name, arg, op, pool, globals, functions)
    if st != 0 {
        return st
    }
    if name != "new_instance" {
        return 0
    }
    let argc = 0
    if op["arg2"] != undefined {
        argc = op["arg2"]
    }
    vPushS(S, vNewInstanceS(S, arg, argc, pool, globals, functions))
    vBumpIpS(S)
    return 1
}
""",
)

w(
    "vm_run_ops_step.kab",
    """
import "self_host/vm_run_ops_plain_new"
import "self_host/vm_run_call"

pub fn runOpsStepS(S, ops, pool, globals, functions) {
    let op = ops[S["ip"]]
    let name = op["op"]
    let arg = op["arg"]
    let st = runOpsPlainNewS(S, name, arg, op, pool, globals, functions)
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
    "vm_run_session.kab",
    """
pub fn vMakeSession() {
    return {
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
}
""",
)

w(
    "vm_run_wire.kab",
    """
pub fn vWireExec(S, runOps, runFnBody, runMethodBody) {
    S["runOps"] = runOps
    S["runFnBody"] = runFnBody
    S["runMethodBody"] = runMethodBody
}
""",
)

w(
    "vm_run_body.kab",
    """
// H6e/P6b: Kabootar bytecode VM subset body — densifying via session object S.
// Exec glue in vm_run_{call,ops,fn,meth,mod}_*; hooks on S break runFn↔runOps cycles.
// Skip-listed until this leaf self-host-compiles ≤10s.
import "self_host/vm_run_session"
import "self_host/vm_run_wire"
import "self_host/vm_run_fn_body"
import "self_host/vm_run_meth_body"
import "self_host/vm_run_ops_loop"
import "self_host/vm_run_mod_enter"
import "self_host/vm_run_mod_leave"

let S = vMakeSession()

fn runOps(ops, pool, globals, functions) {
    runOpsS(S, ops, pool, globals, functions)
}

fn runFnBody(fnDef, pool, globals, functions, callArgs, caps) {
    return runFnBodyS(S, fnDef, pool, globals, functions, callArgs, caps)
}

fn runMethodBody(meth, pool, globals, functions, recv, callArgs) {
    return runMethodBodyS(S, meth, pool, globals, functions, recv, callArgs)
}

vWireExec(S, runOps, runFnBody, runMethodBody)

pub fn runModuleImplBody(bc) {
    let ctx = vModEnter(S, bc)
    S["runOps"](ctx["ops"], ctx["pool"], ctx["globals"], ctx["functions"])
    return vModLeave(S, ctx["old"])
}
""",
)

print("done")
