#!/usr/bin/env python3
"""Split emit_expr_call.kab into ≤~45-line parts at brace-safe boundaries."""
from __future__ import annotations

import re
from pathlib import Path

SH = Path("self_host")
SRC = SH / "emit_expr_call.kab"
MAX_LINES = 45


def main() -> None:
    text = SRC.read_text(encoding="utf-8")
    lines = text.splitlines(True)

    # Find pub fn emitExpr_call body
    start = None
    for i, line in enumerate(lines):
        if line.startswith("pub fn emitExpr_call"):
            start = i
            break
    assert start is not None

    depth = 0
    body_start = None
    end = None
    for i in range(start, len(lines)):
        depth += lines[i].count("{") - lines[i].count("}")
        if body_start is None and depth >= 1:
            body_start = i + 1  # first line inside fn
        if body_start is not None and depth == 0:
            end = i  # closing brace of fn
            break
    assert body_start is not None and end is not None

    header = "".join(lines[:start])
    # Keep shared lets/imports in header; strip old fn
    # Body lines: from after kind-check through before final return true / closing
    body = lines[body_start:end]
    # Drop leading kind check (first if != AST_CALL) — keep in dispatcher
    # Find end of kind guard
    guard_end = 0
    d = 0
    for i, line in enumerate(body):
        if i == 0 and "kind != AST_CALL" in line:
            d += line.count("{") - line.count("}")
            guard_end = i + 1
            while guard_end < len(body) and d > 0:
                d += body[guard_end].count("{") - body[guard_end].count("}")
                guard_end += 1
            break

    work = body[guard_end:]
    # Remove trailing `return true` if present (dispatcher adds it)
    while work and work[-1].strip() in ("", "return true"):
        if work[-1].strip() == "return true":
            work = work[:-1]
            break
        work = work[:-1]

    # Find brace-safe split points (depth relative to work start == 0)
    rel = 0
    safe_after: list[int] = []  # indices in work AFTER which we may split
    for i, line in enumerate(work):
        rel += line.count("{") - line.count("}")
        if rel == 0:
            safe_after.append(i)

    parts: list[list[str]] = []
    cur: list[str] = []
    last_safe = -1
    for i, line in enumerate(work):
        cur.append(line)
        if i in safe_after and len(cur) >= MAX_LINES:
            parts.append(cur)
            cur = []
            last_safe = i
    if cur:
        if parts and len(cur) < 12:
            parts[-1].extend(cur)
        else:
            parts.append(cur)

    # Rewrite return true → set done flag
    def rewrite(part_lines: list[str]) -> str:
        out = []
        for line in part_lines:
            if re.match(r"^[ \t]*return true[ \t]*$", line):
                indent = line[: len(line) - len(line.lstrip())]
                out.append(f'{indent}E["_callDone"] = 1\n')
                out.append(f"{indent}return\n")
            else:
                out.append(line)
        return "".join(out)

    imports = """import "self_host/ast_defs"
import "self_host/emit_defs"
import "self_host/emit_hooks"
import "self_host/emit_sym_index"
import "self_host/emit_arr_util"
import "self_host/emit_op"
import "self_host/emit_sym"
import "self_host/emit_local_map"
import "self_host/emit_call_callee"
import "self_host/emit_drop_callee"
import "self_host/emit_call_args"
import "self_host/emit_try_len"

let eCallRet = 0
let eCallRet2 = 0
let eMemObj = null
let eMemFld = null
let eMemTypeArgs = null
let eMi = 0
let eMeth = null
let eMethTypeParams = null

"""

    part_names: list[str] = []
    for n, part in enumerate(parts, start=1):
        name = f"emitExpr_call_p{n}"
        part_names.append(name)
        fname = f"emit_expr_call_p{n}.kab"
        body_txt = rewrite(part)
        # Dedent one level if consistently over-indented
        content = (
            imports
            + f"pub fn {name}(E) {{\n"
            + body_txt
            + "}\n"
        )
        (SH / fname).write_text(content, encoding="utf-8", newline="\n")
        print(f"wrote {fname} ({len(content.splitlines())} lines)")

    # Thin dispatcher
    calls = "\n".join(
        f"""    {pn}(E)
    if E["_callDone"] == 1 {{
        return true
    }}"""
        for pn in part_names
    )
    imp_parts = "\n".join(f'import "self_host/emit_expr_call_p{i}"' for i in range(1, len(parts) + 1))
    dispatch = f"""import "self_host/ast_defs"
import "self_host/emit_defs"
{imp_parts}

pub fn emitExpr_call(E) {{
    if E["eNode"].kind != AST_CALL {{
        return false
    }}
    E["_callDone"] = 0
{calls}
    return true
}}
"""
    SRC.write_text(dispatch, encoding="utf-8", newline="\n")
    print(f"wrote emit_expr_call.kab dispatcher ({len(parts)} parts)")


if __name__ == "__main__":
    main()
