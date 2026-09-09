#!/usr/bin/env python3
"""Split call_n; hold-box S so wrappers live outside body (P6b)."""
from pathlib import Path

SH = Path("self_host")


def w(name: str, text: str) -> None:
    p = SH / name
    p.write_text(text.lstrip("\n"), encoding="utf-8", newline="\n")
    print("wrote", p)


w(
    "vm_run_call_n_prep.kab",
    """
import "self_host/vm_s_pop"
import "self_host/vm_run_args_pop"
import "self_host/vm_run_callee"

pub fn vCallNPrepS(S, arg) {
    let callee = vPopS(S)
    let ordered = vPopOrderedArgs(S, arg)
    let c = vClassifyCallee(callee)
    if c["kind"] == "bad" {
        S["unsupported"] = true
        return { "st": 2 }
    }
    return { "st": 0, "c": c, "ordered": ordered }
}
""",
)

w(
    "vm_run_call_n.kab",
    """
import "self_host/vm_s_push"
import "self_host/vm_s_ip"
import "self_host/vm_run_call_n_prep"
import "self_host/vm_run_call_apply"

pub fn runOpCallNS(S, name, arg, pool, globals, functions) {
    if name != "call" {
        return 0
    }
    let p = vCallNPrepS(S, arg)
    if p["st"] == 2 {
        return 2
    }
    vPushS(S, vApplyCalleeS(S, p["c"], p["ordered"], pool, globals, functions))
    vBumpIpS(S)
    return 1
}
""",
)

w(
    "vm_run_s_hold.kab",
    """
/// Shared session box so wrap_* leaves can close over S without nested fns.
let BOX = { "S": null }

pub fn vSetBoxS(S) {
    BOX["S"] = S
}

pub fn vBoxS() {
    return BOX["S"]
}
""",
)

w(
    "vm_run_wrap_ops.kab",
    """
import "self_host/vm_run_s_hold"
import "self_host/vm_run_ops_loop"

pub fn runOps(ops, pool, globals, functions) {
    runOpsS(vBoxS(), ops, pool, globals, functions)
}
""",
)

w(
    "vm_run_wrap_fn.kab",
    """
import "self_host/vm_run_s_hold"
import "self_host/vm_run_fn_body"

pub fn runFnBody(fnDef, pool, globals, functions, callArgs, caps) {
    return runFnBodyS(vBoxS(), fnDef, pool, globals, functions, callArgs, caps)
}
""",
)

w(
    "vm_run_wrap_meth.kab",
    """
import "self_host/vm_run_s_hold"
import "self_host/vm_run_meth_body"

pub fn runMethodBody(meth, pool, globals, functions, recv, callArgs) {
    return runMethodBodyS(vBoxS(), meth, pool, globals, functions, recv, callArgs)
}
""",
)

w(
    "vm_run_body.kab",
    """
// H6e/P6b: Kabootar bytecode VM subset body — densifying via session object S.
// Wrappers in vm_run_wrap_*; hooks on S break runFn↔runOps cycles.
// Skip-listed until this leaf self-host-compiles ≤10s.
import "self_host/vm_run_session"
import "self_host/vm_run_s_hold"
import "self_host/vm_run_wire"
import "self_host/vm_run_wrap_ops"
import "self_host/vm_run_wrap_fn"
import "self_host/vm_run_wrap_meth"
import "self_host/vm_run_mod_enter"
import "self_host/vm_run_mod_leave"

let S = vMakeSession()
vSetBoxS(S)
vWireExec(S, runOps, runFnBody, runMethodBody)

pub fn runModuleImplBody(bc) {
    let ctx = vModEnter(S, bc)
    S["runOps"](ctx["ops"], ctx["pool"], ctx["globals"], ctx["functions"])
    return vModLeave(S, ctx["old"])
}
""",
)

# Remove abandoned exec_core if present
p = SH / "vm_run_exec_core.kab"
if p.exists():
    p.unlink()
    print("removed vm_run_exec_core.kab")

print("done")
