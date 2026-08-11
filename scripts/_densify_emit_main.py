#!/usr/bin/env python3
"""P6b densify: split self_host/emit_main.kab into ≤10s self-host shards.

Strategy (VM-like):
  - Pure helpers → small pub modules
  - emitExpr / emitStmt → kind handlers that return true if handled
  - Mutual recursion via E[\"tramp\"] + emit_hooks (eCallExpr / eCallStmt)
  - Module-level session E in emit_exec (like vm_run_exec_core)

Run from repo root:
  python scripts/_densify_emit_main.py
"""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SH = ROOT / "self_host"
SRC = SH / "emit_main.kab"


def write(name: str, text: str) -> None:
    p = SH / name
    p.write_text(text.lstrip("\n"), encoding="utf-8", newline="\n")
    print(f"wrote {p.relative_to(ROOT)} ({len(text.splitlines())} lines)")


def fn_bodies(lines: list[str]) -> list[tuple[str, int, int, str]]:
    """Return (name, start0, end0_inclusive, kind) for top-level fn/pub fn."""
    out: list[tuple[str, int, int, str]] = []
    depth = 0
    start = -1
    name = ""
    kind = ""
    for i, line in enumerate(lines):
        stripped = line.lstrip()
        if depth == 0 and (
            stripped.startswith("fn ") or stripped.startswith("pub fn ")
        ):
            kind = "pub" if stripped.startswith("pub fn ") else "fn"
            m = re.match(r"(?:pub )?fn (\w+)", stripped)
            assert m, stripped
            name = m.group(1)
            start = i
        depth += line.count("{") - line.count("}")
        if start >= 0 and depth == 0:
            out.append((name, start, i, kind))
            start = -1
    return out


def replace_calls(body: str) -> str:
    """Route recursive emit through hooks; keep same-module helper names as imports."""
    body = re.sub(r"\bemitExpr\(", "eCallExpr(", body)
    body = re.sub(r"\bemitStmt\(", "eCallStmt(", body)
    # Don't rewrite the body function definitions themselves if present
    body = body.replace("fn eCallExpr(", "fn emitExpr(")
    body = body.replace("fn eCallStmt(", "fn emitStmt(")
    body = body.replace("pub fn eCallExpr(", "pub fn emitExpr(")
    body = body.replace("pub fn eCallStmt(", "pub fn emitStmt(")
    return body


def make_pub(block: str) -> str:
    if block.lstrip().startswith("pub fn "):
        return block
    return re.sub(r"^fn ", "pub fn ", block, count=1, flags=re.M)


IMPORTS_BASE = """\
import "self_host/ast_defs"
import "self_host/emit_defs"
"""

IMPORTS_HOOKS = """\
import "self_host/emit_hooks"
"""


def main() -> None:
    text = SRC.read_text(encoding="utf-8")
    lines = text.splitlines(True)
    fns = {n: (a, b) for n, a, b, _ in fn_bodies(lines)}

    def body_of(name: str) -> str:
        a, b = fns[name]
        return make_pub("".join(lines[a : b + 1]))

    # --- hooks (no expr/stmt imports) ---
    write(
        "emit_hooks.kab",
        """
// P6b: trampoline hooks for emitExpr/emitStmt across shards (no circular imports).
pub fn eCallExpr(E, node) {
    let prevH = E["_hook"]
    let prevN = E["_node"]
    E["_hook"] = 0
    E["_node"] = node
    E["tramp"]()
    E["_hook"] = prevH
    E["_node"] = prevN
}

pub fn eCallStmt(E, node) {
    let prevH = E["_hook"]
    let prevN = E["_node"]
    E["_hook"] = 1
    E["_node"] = node
    E["tramp"]()
    E["_hook"] = prevH
    E["_node"] = prevN
}
""",
    )

    # --- small helpers (no emitExpr) ---
    write(
        "emit_const_key.kab",
        IMPORTS_BASE
        + "\n"
        + body_of("constKey")
        + "\n",
    )
    write(
        "emit_local_map.kab",
        """
"""
        + body_of("resetLocalMap")
        + "\n"
        + body_of("rebuildLocalMap")
        + "\n",
    )
    write(
        "emit_sym_index.kab",
        IMPORTS_BASE
        + 'import "self_host/emit_const_key"\n\n'
        + body_of("symIndex")
        + "\n",
    )
    write(
        "emit_arr_util.kab",
        body_of("arrUtil") + "\n",
    )
    write(
        "emit_op.kab",
        body_of("emitOp") + "\n",
    )
    write(
        "emit_local_sym.kab",
        body_of("localSymIndex") + "\n",
    )

    # emitSym uses eCallRet scratch + symIndex + emitOp
    emit_sym = body_of("emitSym")
    write(
        "emit_sym.kab",
        IMPORTS_BASE
        + """import "self_host/emit_sym_index"
import "self_host/emit_op"

let eCallRet = 0

"""
        + emit_sym
        + "\n",
    )

    write(
        "emit_call_callee.kab",
        IMPORTS_HOOKS + "\n" + replace_calls(body_of("emitCallCallee")) + "\n",
    )
    write(
        "emit_drop_callee.kab",
        body_of("dropCallCallee") + "\n",
    )
    write(
        "emit_call_args.kab",
        IMPORTS_HOOKS + "\n" + replace_calls(body_of("emitCallArgExprs")) + "\n",
    )

    # try* helpers
    write(
        "emit_try_array_push.kab",
        IMPORTS_BASE
        + """import "self_host/emit_op"
import "self_host/emit_local_sym"
import "self_host/emit_sym_index"
import "self_host/emit_hooks"

"""
        + replace_calls(body_of("tryEmitArrayPushAssign"))
        + "\n",
    )
    write(
        "emit_try_len.kab",
        IMPORTS_BASE
        + """import "self_host/emit_op"
import "self_host/emit_local_sym"
import "self_host/emit_sym_index"

"""
        + body_of("tryEmitLenCall")
        + "\n",
    )
    write(
        "emit_try_acc_add.kab",
        IMPORTS_BASE
        + """import "self_host/emit_op"
import "self_host/emit_local_sym"
import "self_host/emit_sym_index"
import "self_host/emit_hooks"

"""
        + replace_calls(body_of("tryEmitAccAddAssign"))
        + "\n",
    )

    # Jump helpers
    write(
        "emit_patch_jump.kab",
        """
let eCallRet = 0

"""
        + body_of("patchRelJump")
        + "\n"
        + body_of("pushJmpAtLen")
        + "\n",
    )

    write(
        "emit_if_stmt.kab",
        IMPORTS_BASE
        + """import "self_host/emit_op"
import "self_host/emit_hooks"
import "self_host/emit_patch_jump"

let eCallRet = 0

"""
        + replace_calls(body_of("emitIfStmt"))
        + "\n",
    )

    # --- Split emitExpr into kind handlers ---
    expr_a, expr_b = fns["emitExpr"]
    expr_lines = lines[expr_a : expr_b + 1]
    # Skip signature + opening; extract kind blocks at depth 1 inside function
    handlers = extract_kind_handlers(expr_lines, "E[\"eNode\"].kind", "emitExpr")
    handler_names: list[str] = []
    for kind, block in handlers:
        fname = f"emit_expr_{kind_to_snake(kind)}.kab"
        hname = f"emitExpr_{kind_to_snake(kind)}"
        handler_names.append(hname)
        # block is the if-body without the if line — wrap as pub fn returning true
        wrapped = wrap_kind_handler(hname, kind, block, is_expr=True)
        deps = infer_imports(wrapped)
        write(fname, deps + wrapped + "\n")

    # Dispatcher for emitExpr
    disp_imports = "\n".join(f'import "self_host/emit_expr_{kind_to_snake(k)}"' for k, _ in handlers)
    disp_body = "\n".join(
        f"""    if {hn}(E) {{
        return
    }}"""
        for hn in handler_names
    )
    write(
        "emit_expr_body.kab",
        IMPORTS_BASE
        + disp_imports
        + """

pub fn emitExprBody(E, node) {
    E["eNode"] = node
    if E["eNode"] == null {
        throw "emitExpr: null node"
    }
"""
        + disp_body
        + """
    throw "Unsupported expr: " + E["eNode"].kind
}
""",
    )

    # --- Split emitStmt ---
    stmt_a, stmt_b = fns["emitStmt"]
    stmt_lines = lines[stmt_a : stmt_b + 1]
    stmt_handlers = extract_stmt_handlers(stmt_lines)
    stmt_names: list[str] = []
    for kind, block in stmt_handlers:
        snake = kind_to_snake(kind)
        hname = f"emitStmt_{snake}"
        stmt_names.append(hname)
        wrapped = wrap_kind_handler(hname, kind, block, is_expr=False)
        deps = infer_imports(wrapped)
        # if stmt uses emitIfStmt specially
        if "emitIfStmt" in wrapped or "eCallIf" in wrapped:
            deps += 'import "self_host/emit_if_stmt"\n'
        write(f"emit_stmt_{snake}.kab", deps + wrapped + "\n")

    stmt_disp_imports = "\n".join(
        f'import "self_host/emit_stmt_{kind_to_snake(k)}"' for k, _ in stmt_handlers
    )
    stmt_disp_body = "\n".join(
        f"""    if {hn}(E) {{
        return
    }}"""
        for hn in stmt_names
    )
    write(
        "emit_stmt_body.kab",
        IMPORTS_BASE
        + 'import "self_host/emit_if_stmt"\n'
        + stmt_disp_imports
        + """

pub fn emitStmtBody(E, node) {
    E["eStmtNode"] = node
    E["eNode"] = node
    if E["eStmtNode"] == null {
        throw "emitStmt: null node"
    }
    E["eStmtKind"] = E["eStmtNode"].kind
    if E["eStmtKind"] == AST_IF {
        emitIfStmt(E)
        return
    }
"""
        + stmt_disp_body
        + """
    if E["eNode"].kind == undefined {
        throw "emitStmt: missing kind"
    }
    throw "Unsupported stmt: " + E["eStmtKind"]
}
""",
    )

    write(
        "emit_tramp.kab",
        """
import "self_host/emit_expr_body"
import "self_host/emit_stmt_body"

pub fn eTramp(E) {
    if E["_hook"] == 0 {
        return emitExprBody(E, E["_node"])
    }
    return emitStmtBody(E, E["_node"])
}
""",
    )

    # emitMain body with eCallStmt + imported helpers
    main_body = replace_calls(body_of("emitMain"))
    main_body = main_body.replace("resetLocalMap(", "resetLocalMap(")
    write(
        "emit_main_fn.kab",
        IMPORTS_BASE
        + """import "self_host/emit_local_map"
import "self_host/emit_arr_util"
import "self_host/emit_op"
import "self_host/emit_hooks"

"""
        + main_body
        + "\n",
    )

    # Module-level session + wire (VM pattern)
    write(
        "emit_exec.kab",
        """
// P6b: module session + trampoline (CI-fast leaf; heavy logic in shards).
import "self_host/emit_session"
import "self_host/emit_tramp"
import "self_host/emit_main_fn"

let E = eMakeSession()

fn tramp() {
    return eTramp(E)
}

E["tramp"] = tramp
E["_hook"] = 0
E["_node"] = null

pub fn emitMainExec(program) {
    eResetSession(E)
    E["tramp"] = tramp
    E["_hook"] = 0
    E["_node"] = null
    return emitMain(E, program)
}
""",
    )

    # Thin emit_main re-export for existing imports
    write(
        "emit_main.kab",
        """
// P6b densify: re-export session-wired emitter (shards under emit_*).
import "self_host/emit_exec"

pub let emitMainFromExec = emitMainExec

// Compat: old call shape emitMain(E, program) — ignore passed E, use module session.
pub fn emitMain(Eignored, program) {
    return emitMainExec(program)
}
""",
    )

    write(
        "emit_impl.kab",
        """
// H6e: self-hosted bytecode emitter body — thin driver (skip-listed).
import "self_host/emit_exec"

pub fn emitImpl(program) {
    return emitMainExec(program)
}
""",
    )

    print("\nDone. Measure shards with scripts/_emit_compile_calibrate.py / profile_emit_compile.py")


def kind_to_snake(kind: str) -> str:
    k = kind.strip()
    if k.startswith("AST_"):
        k = k[4:]
    return k.lower()


def extract_kind_handlers(
    fn_lines: list[str], kind_expr: str, _fn_name: str
) -> list[tuple[str, str]]:
    """Extract top-level `if E[\"eNode\"].kind == AST_X { ... }` bodies inside a function."""
    handlers: list[tuple[str, str]] = []
    depth = 0
    i = 0
    # Enter function body (depth becomes 1 at the opening `{` line).
    while i < len(fn_lines):
        line = fn_lines[i]
        depth += line.count("{") - line.count("}")
        i += 1
        if depth >= 1:
            break
    base = 1
    while i < len(fn_lines):
        line = fn_lines[i]
        stripped = line.strip()
        m = re.match(
            r'if\s+E\["eNode"\]\.kind\s*==\s*(AST_\w+)\s*\{',
            stripped,
        )
        if m and depth == base:
            kind = m.group(1)
            start_depth = depth
            depth += line.count("{") - line.count("}")
            block_lines: list[str] = []
            i += 1
            while i < len(fn_lines) and depth > start_depth:
                block_lines.append(fn_lines[i])
                depth += fn_lines[i].count("{") - fn_lines[i].count("}")
                i += 1
            handlers.append((kind, "".join(block_lines)))
            continue
        depth += line.count("{") - line.count("}")
        i += 1
    # Drop the closing `}` of each if that was included in the body scan.
    cleaned: list[tuple[str, str]] = []
    for kind, block in handlers:
        bl = block.rstrip()
        if bl.endswith("}"):
            # remove last line if it is only a closing brace
            parts = bl.splitlines(True)
            if parts and parts[-1].strip() == "}":
                parts = parts[:-1]
                bl = "".join(parts)
        cleaned.append((kind, bl + ("\n" if bl and not bl.endswith("\n") else "")))
    return cleaned


def extract_stmt_handlers(fn_lines: list[str]) -> list[tuple[str, str]]:
    """Like expr, but skip AST_IF (handled via emitIfStmt) and use E[\"eNode\"].kind."""
    handlers = extract_kind_handlers(fn_lines, 'E["eNode"].kind', "emitStmt")
    # Also catch E["eStmtKind"] == AST_IF — skip
    return [(k, b) for k, b in handlers if k != "AST_IF"]


def rewrite_bare_returns(block: str) -> str:
    """Bare `return` in emitExpr/emitStmt means 'handled' → `return true` for handlers."""
    out_lines: list[str] = []
    for line in block.splitlines(True):
        if re.match(r"^[ \t]*return[ \t]*\n?$", line):
            indent = line[: len(line) - len(line.lstrip())]
            out_lines.append(f"{indent}return true\n")
        else:
            out_lines.append(line)
    return "".join(out_lines)


def wrap_kind_handler(hname: str, kind: str, block: str, is_expr: bool) -> str:
    del is_expr  # reserved for future expr/stmt differences
    block = rewrite_bare_returns(replace_calls(block))
    return f"""
pub fn {hname}(E) {{
    if E["eNode"].kind != {kind} {{
        return false
    }}
{block.rstrip()}
    return true
}}
"""


def infer_imports(code: str) -> str:
    imps: list[str] = [IMPORTS_BASE.rstrip(), 'import "self_host/emit_hooks"']
    mapping = [
        ("constKey(", "emit_const_key"),
        ("symIndex(", "emit_sym_index"),
        ("arrUtil(", "emit_arr_util"),
        ("emitOp(", "emit_op"),
        ("emitSym(", "emit_sym"),
        ("localSymIndex(", "emit_local_sym"),
        ("resetLocalMap(", "emit_local_map"),
        ("rebuildLocalMap(", "emit_local_map"),
        ("emitCallCallee(", "emit_call_callee"),
        ("dropCallCallee(", "emit_drop_callee"),
        ("emitCallArgExprs(", "emit_call_args"),
        ("tryEmitArrayPushAssign(", "emit_try_array_push"),
        ("tryEmitLenCall(", "emit_try_len"),
        ("tryEmitAccAddAssign(", "emit_try_acc_add"),
        ("patchRelJump(", "emit_patch_jump"),
        ("pushJmpAtLen(", "emit_patch_jump"),
        ("emitIfStmt(", "emit_if_stmt"),
    ]
    for needle, mod in mapping:
        if needle in code:
            imps.append(f'import "self_host/{mod}"')
    # eCallRet scratch if assignment present
    head = "\n".join(dict.fromkeys(imps)) + "\n\n"
    if re.search(r"\beCallRet\b", code) or re.search(r"\beCallRet2\b", code):
        head += "let eCallRet = 0\nlet eCallRet2 = 0\n\n"
    return head


if __name__ == "__main__":
    main()
