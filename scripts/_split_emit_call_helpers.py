#!/usr/bin/env python3
"""Recursively extract large if-blocks from emit_expr_call until leaves ≤ MAX lines."""
from __future__ import annotations

import re
from pathlib import Path

SH = Path("self_host")
SRC = SH / "emit_expr_call.kab"
BACKUP = SH / "_emit_expr_call_pre_split.kab"
MAX = 50
COUNTER = 0

IMPORTS = """import "self_host/ast_defs"
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


def rewrite_done(body: str) -> str:
    out = []
    for line in body.splitlines(True):
        if re.match(r"^[ \t]*return true[ \t]*$", line):
            ind = line[: len(line) - len(line.lstrip())]
            out.append(f'{ind}E["_callDone"] = 1\n')
            out.append(f"{ind}return\n")
        else:
            out.append(line)
    return "".join(out)


def find_large_ifs(body_lines: list[str]) -> list[tuple[int, int]]:
    """If-blocks at depth 0 whose span ≥ MAX."""
    depth = 0
    found: list[tuple[int, int]] = []
    i = 0
    while i < len(body_lines):
        line = body_lines[i]
        stripped = line.strip()
        if stripped.startswith("if ") and stripped.endswith("{") and depth == 0:
            start = i
            sd = depth
            depth += line.count("{") - line.count("}")
            i += 1
            while i < len(body_lines) and depth > sd:
                depth += body_lines[i].count("{") - body_lines[i].count("}")
                i += 1
            if (i - 1) - start + 1 >= MAX:
                found.append((start, i - 1))
            continue
        depth += line.count("{") - line.count("}")
        i += 1
    return found


def dedent(lines: list[str], n: int = 4) -> list[str]:
    out = []
    for ln in lines:
        if ln.startswith(" " * n):
            out.append(ln[n:])
        else:
            out.append(ln)
    return out


def extract_from_body(body_lines: list[str], imports_extra: list[str]) -> tuple[list[str], list[str]]:
    """Replace large ifs with helper calls; return (new_body, list of helper filenames written)."""
    global COUNTER
    written: list[str] = []
    body = list(body_lines)
    # largest first
    blocks = sorted(find_large_ifs(body), key=lambda x: x[0] - x[1])
    # process from end
    for start, end in sorted(blocks, key=lambda x: -x[0]):
        block = body[start : end + 1]
        cond_m = re.match(r"(\s*)if (.+) \{", block[0].rstrip("\n"))
        if not cond_m:
            continue
        indent, cond = cond_m.group(1), cond_m.group(2)
        inner = dedent(block[1:-1])
        COUNTER += 1
        hname = f"emitExpr_call_h{COUNTER}"
        fname = f"emit_expr_call_h{COUNTER}.kab"
        # Recursively shrink helper body first
        h_body_lines = inner
        h_body_lines, more = extract_from_body(h_body_lines, imports_extra)
        written.extend(more)
        for m in more:
            mod = m.replace(".kab", "")
            if f'import "self_host/{mod}"' not in imports_extra:
                imports_extra.append(f'import "self_host/{mod}"')
        htext = (
            IMPORTS
            + "\n".join(imports_extra)
            + ("\n" if imports_extra else "")
            + f"\npub fn {hname}(E) {{\n"
            + rewrite_done("".join(h_body_lines))
            + "}\n"
        )
        (SH / fname).write_text(htext, encoding="utf-8", newline="\n")
        written.append(fname)
        print(f"wrote {fname} ({len(htext.splitlines())} lines)")
        repl = [
            f"{indent}if {cond} {{\n",
            f"{indent}    {hname}(E)\n",
            f'{indent}    if E["_callDone"] == 1 {{\n',
            f"{indent}        return true\n",
            f"{indent}    }}\n",
            f"{indent}}}\n",
        ]
        body = body[:start] + repl + body[end + 1 :]
        imports_extra.append(f'import "self_host/{fname[:-4]}"')
    return body, written


def main() -> None:
    global COUNTER
    COUNTER = 0
    # clean old helpers
    for p in SH.glob("emit_expr_call_h*.kab"):
        p.unlink()
    for p in SH.glob("emit_expr_call_p*.kab"):
        p.unlink()

    src = BACKUP if BACKUP.exists() else SRC
    text = src.read_text(encoding="utf-8")
    lines = text.splitlines(True)
    fn_i = next(i for i, l in enumerate(lines) if "pub fn emitExpr_call" in l)
    depth = 0
    body0 = None
    fn_end = None
    for i in range(fn_i, len(lines)):
        depth += lines[i].count("{") - lines[i].count("}")
        if body0 is None and "{" in lines[i]:
            body0 = i + 1
        if body0 is not None and depth == 0:
            fn_end = i
            break
    body = lines[body0:fn_end]

    # strip kind guard into dispatcher
    g = 0
    d = 0
    if body and "kind != AST_CALL" in body[0]:
        d += body[0].count("{") - body[0].count("}")
        g = 1
        while g < len(body) and d > 0:
            d += body[g].count("{") - body[g].count("}")
            g += 1
    work = body[g:]
    # drop trailing return true
    while work and work[-1].strip() in ("return true", ""):
        if work[-1].strip() == "return true":
            work = work[:-1]
            break
        work = work[:-1]

    extra_imports: list[str] = []
    new_body, written = extract_from_body(work, extra_imports)
    # unique imports preserve order
    seen = set()
    uniq = []
    for im in extra_imports:
        if im not in seen:
            seen.add(im)
            uniq.append(im)

    dispatch = (
        """import "self_host/ast_defs"
import "self_host/emit_defs"
import "self_host/emit_hooks"
import "self_host/emit_drop_callee"
import "self_host/emit_try_len"
"""
        + "\n".join(uniq)
        + """

let eCallRet = 0
let eCallRet2 = 0
let eMemObj = null
let eMemFld = null
let eMemTypeArgs = null
let eMi = 0
let eMeth = null
let eMethTypeParams = null

pub fn emitExpr_call(E) {
    if E["eNode"].kind != AST_CALL {
        return false
    }
    E["_callDone"] = 0
"""
        + rewrite_done("".join(new_body))
        + "    return true\n}\n"
    )
    SRC.write_text(dispatch, encoding="utf-8", newline="\n")
    print(f"dispatcher lines={len(dispatch.splitlines())} helpers={len(written)}")
    # Report oversized leaves
    for p in sorted(SH.glob("emit_expr_call*.kab")):
        n = len(p.read_text(encoding="utf-8").splitlines())
        flag = " ***" if n > MAX + 30 else ""
        print(f"  {p.name}: {n}{flag}")


if __name__ == "__main__":
    main()
