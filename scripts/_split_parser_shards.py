#!/usr/bin/env python3
"""P6b: split oversized parser shards (stmt/postfix/compare/add_shift).

Run from repo root after _densify_parser_impl.py:
  python scripts/_split_parser_shards.py
"""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SH = ROOT / "self_host"

IMPORTS = """import "self_host/lexer_defs"
import "self_host/ast_defs"
import "self_host/parser_hooks"
import "self_host/parser_util"
"""


def write(name: str, text: str) -> None:
    p = SH / name
    p.write_text(text.lstrip("\n"), encoding="utf-8", newline="\n")
    print(f"wrote {p.relative_to(ROOT)} ({len(text.splitlines())} lines)")


def fn_body_lines(path: Path, fn_name: str) -> list[str]:
    text = path.read_text(encoding="utf-8")
    m = re.search(rf"pub fn {fn_name}\(sess\) \{{", text)
    assert m, fn_name
    start = text[: m.end()].count("\n")
    lines = text.splitlines()
    depth = 0
    body_start = start
    for i in range(start, len(lines)):
        depth += lines[i].count("{") - lines[i].count("}")
        if i > start and depth == 0:
            return lines[body_start + 1 : i]  # inside fn, exclude outer braces
    raise SystemExit(f"unclosed {fn_name}")


def dedent_block(lines: list[str]) -> list[str]:
    if not lines:
        return lines
    indent = len(lines[0]) - len(lines[0].lstrip())
    out = []
    for ln in lines:
        if ln.strip() == "":
            out.append("")
        elif ln.startswith(" " * indent):
            out.append(ln[indent:])
        else:
            out.append(ln.lstrip())
    return out


def wrap_handler(name: str, cond: str, body_lines: list[str], ret_null: str = "return null") -> str:
    body = "\n".join("    " + ln if ln else "" for ln in body_lines)
    return f"""pub fn {name}(sess) {{
    if !({cond}) {{
        {ret_null}
    }}
{body}
}}
"""


SPLIT_MARKERS: dict[str, str] = {
    "parser_postfix.kab": "parser_postfix_lit",
    "parser_compare.kab": "parser_compare_eq",
    "parser_add_shift.kab": "parser_add_shift_plus",
    "parser_stmt.kab": "parser_stmt_let",
}


def already_split(name: str) -> bool:
    marker = SPLIT_MARKERS.get(name)
    if not marker:
        return False
    return f'import "self_host/{marker}"' in (SH / name).read_text(encoding="utf-8")


def split_postfix() -> None:
    if already_split("parser_postfix.kab"):
        print("skip parser_postfix (already split)")
        return
    body = fn_body_lines(SH / "parser_postfix.kab", "parsePostfix")
    # body[0] is EOF check; [1-2] init pLeft/pTypeArgs
    lit = dedent_block(body[3:94])       # through this/self ident primary
    paren = dedent_block(body[94:174])   # ( ... ) group/arrow
    obj_arr = dedent_block(body[174:221])  # { } and [ ]
    bare = dedent_block(body[224:254])   # bare arrow =>
    tail = dedent_block(body[254:356])   # postfix loop + return

    write("parser_postfix_lit.kab", IMPORTS + "\n" + wrap_handler("parsePostfix_lit", "true", lit, "return 0") + "\n")
    write("parser_postfix_paren.kab", IMPORTS + "\n" + wrap_handler(
        "parsePostfix_paren", 'sess["pLeft"] == null', paren, "return 0") + "\n")
    write("parser_postfix_obj_arr.kab", IMPORTS + "\n" + wrap_handler(
        "parsePostfix_obj_arr", 'sess["pLeft"] == null', obj_arr, "return 0") + "\n")
    write("parser_postfix_bare_arrow.kab", IMPORTS + "\n" + f"""pub fn parsePostfix_bareArrow(sess) {{
{chr(10).join("    " + ln if ln else "" for ln in bare)}
    return 0
}}
""")
    write("parser_postfix_tail.kab", IMPORTS + "\n" + f"""pub fn parsePostfix_tail(sess) {{
{chr(10).join("    " + ln if ln else "" for ln in tail)}
}}
""")

    write(
        "parser_postfix.kab",
        IMPORTS
        + """
import "self_host/parser_postfix_lit"
import "self_host/parser_postfix_paren"
import "self_host/parser_postfix_obj_arr"
import "self_host/parser_postfix_bare_arrow"
import "self_host/parser_postfix_tail"

pub fn parsePostfix(sess) {
    if sess["pCur"].type == "EOF" {
        throw "parsePostfix EOF"
    }
    sess["pLeft"] = null
    sess["pTypeArgs"] = []
    parsePostfix_lit(sess)
    if sess["pLeft"] == null {
        parsePostfix_paren(sess)
    }
    if sess["pLeft"] == null {
        parsePostfix_obj_arr(sess)
    }
    if sess["pLeft"] == null {
        throw json_stringify(sess["pCur"])
    }
    parsePostfix_bareArrow(sess)
    return parsePostfix_tail(sess)
}
""",
    )


def split_compare() -> None:
    if already_split("parser_compare.kab"):
        print("skip parser_compare (already split)")
        return
    body = fn_body_lines(SH / "parser_compare.kab", "parseCompare")
    head = dedent_block(body[0:8])   # EOF, pLeft=addShift, pInAddSub early out
    eq = dedent_block(body[8:15])
    bit = dedent_block(body[15:35])
    logic = dedent_block(body[38:79])

    write("parser_compare_eq.kab", IMPORTS + "\n" + f"""pub fn parseCompare_eq(sess) {{
{chr(10).join("    " + ln if ln else "" for ln in eq)}
    return 0
}}
""")
    write("parser_compare_bit.kab", IMPORTS + "\n" + f"""pub fn parseCompare_bit(sess) {{
{chr(10).join("    " + ln if ln else "" for ln in bit)}
    return 0
}}
""")
    write("parser_compare_logic.kab", IMPORTS + "\n" + f"""pub fn parseCompare_logic(sess) {{
{chr(10).join("    " + ln if ln else "" for ln in logic)}
    return 0
}}
""")

    write(
        "parser_compare.kab",
        IMPORTS
        + """
import "self_host/parser_compare_eq"
import "self_host/parser_compare_bit"
import "self_host/parser_compare_logic"

pub fn parseCompare(sess) {
"""
        + "\n".join("    " + ln if ln else "" for ln in head)
        + """
    parseCompare_eq(sess)
    parseCompare_bit(sess)
    if sess["pNoBit"] == 1 {
        return sess["pLeft"]
    }
    parseCompare_logic(sess)
    return sess["pLeft"]
}
""",
    )


def split_add_shift() -> None:
    if already_split("parser_add_shift.kab"):
        print("skip parser_add_shift (already split)")
        return
    body = fn_body_lines(SH / "parser_add_shift.kab", "parseAddShift")
    init = dedent_block(body[0:1])
    plus_loop = dedent_block(body[1:14])
    shift_loop = dedent_block(body[14:27])
    ret = dedent_block(body[27:28])

    write("parser_add_shift_plus.kab", IMPORTS + "\n" + f"""pub fn parseAddShift_plus(sess) {{
{chr(10).join("    " + ln if ln else "" for ln in plus_loop)}
    return 0
}}
""")
    write("parser_add_shift_shift.kab", IMPORTS + "\n" + f"""pub fn parseAddShift_shift(sess) {{
{chr(10).join("    " + ln if ln else "" for ln in shift_loop)}
    return 0
}}
""")

    write(
        "parser_add_shift.kab",
        IMPORTS
        + """
import "self_host/parser_add_shift_plus"
import "self_host/parser_add_shift_shift"

pub fn parseAddShift(sess) {
"""
        + "\n".join("    " + ln if ln else "" for ln in init)
        + """
    parseAddShift_plus(sess)
    parseAddShift_shift(sess)
"""
        + "\n".join("    " + ln if ln else "" for ln in ret)
        + "\n}\n",
    )


STMT_HANDLER_STARTS: list[tuple[str, str]] = [
    ("parseStmt_let", 'sess["pCur"].type == TOKEN_LET'),
    ("parseStmt_enum", 'sess["pCur"].type == TOKEN_ENUM'),
    ("parseStmt_class", 'sess["pCur"].type == TOKEN_CLASS || sess["pCur"].type == TOKEN_STRUCT'),
    ("parseStmt_iface", 'sess["pCur"].type == TOKEN_INTERFACE || sess["pCur"].type == TOKEN_TRAIT'),
    ("parseStmt_fn", 'sess["pCur"].type == TOKEN_FN'),
    ("parseStmt_if", 'sess["pCur"].type == TOKEN_IF'),
    ("parseStmt_try", 'sess["pCur"].type == TOKEN_TRY'),
    ("parseStmt_for", 'sess["pCur"].type == TOKEN_FOR'),
    ("parseStmt_while", 'sess["pCur"].type == TOKEN_WHILE'),
    ("parseStmt_continue", 'sess["pCur"].type == TOKEN_CONTINUE'),
    ("parseStmt_break", 'sess["pCur"].type == TOKEN_BREAK'),
    ("parseStmt_throw", 'sess["pCur"].type == TOKEN_THROW'),
    ("parseStmt_return", 'sess["pCur"].type == TOKEN_RETURN'),
    ("parseStmt_block", 'sess["pCur"].type == "{"'),
]


def stmt_handler_ranges(lines: list[str]) -> list[tuple[str, str, int, int]]:
    starts: list[tuple[int, str, str]] = []
    for i, ln in enumerate(lines):
        if not re.match(r"^    if sess", ln):
            continue
        if i > 0 and "early IDENT=" in lines[i - 1]:
            starts.append((i, "parseStmt_earlyAssign", 'sess["pCur"].type == TOKEN_IDENT'))
            continue
        matched = False
        for name, cond in STMT_HANDLER_STARTS:
            if cond in ln:
                starts.append((i, name, cond))
                matched = True
                break
        if not matched and 'sess["pCur"].type == TOKEN_IDENT' in ln:
            starts.append((i, "parseStmt_lateAssign", 'sess["pCur"].type == TOKEN_IDENT'))
    out: list[tuple[str, str, int, int]] = []
    for j, (start, name, cond) in enumerate(starts):
        if j + 1 < len(starts):
            end = starts[j + 1][0] - 1
        else:
            end = len(lines) - 1
            for k in range(start + 1, len(lines)):
                if 'sess["pAssignLhs"] = pCallCompare' in lines[k]:
                    end = k - 1
                    break
        while end > start and lines[end].strip() != "}":
            end -= 1
        out.append((name, cond, start, end))
    return out


def strip_outer_if(lines: list[str]) -> list[str]:
    if not lines or not lines[0].strip().startswith("if "):
        return lines
    if lines[-1].strip() == "}":
        return lines[1:-1]
    return lines[1:]


def split_stmt() -> None:
    if already_split("parser_stmt.kab"):
        print("skip parser_stmt (already split)")
        return
    lines = (SH / "parser_stmt.kab").read_text(encoding="utf-8").splitlines()
    handlers = stmt_handler_ranges(lines)
    tail_start = handlers[-1][3] + 1
    while tail_start < len(lines) and lines[tail_start].strip() == "":
        tail_start += 1
    tail_end = len(lines) - 1
    while tail_end > tail_start and lines[tail_end].strip() == "":
        tail_end -= 1
    if lines[tail_end].strip() == "}":
        tail_end -= 1
    tail = dedent_block(lines[tail_start : tail_end + 1])

    imports = []
    dispatch: list[str] = []
    for name, cond, start, end in handlers:
        chunk = dedent_block(lines[start : end + 1])
        chunk = strip_outer_if(chunk)
        fname = name.split("_", 1)[1]
        shard = f"parser_stmt_{fname}.kab"
        write(shard, IMPORTS + "\n" + wrap_handler(name, cond, chunk) + "\n")
        imports.append(f'import "self_host/{shard[:-4]}"')
        dispatch.append(f"    let r = {name}(sess)")
        dispatch.append("    if r != null { return r }")

    write("parser_stmt_expr.kab", IMPORTS + "\n" + f"""pub fn parseStmt_expr(sess) {{
{chr(10).join("    " + ln if ln else "" for ln in tail)}
}}
""")
    imports.append('import "self_host/parser_stmt_expr"')

    write(
        "parser_stmt.kab",
        IMPORTS
        + "\n"
        + "\n".join(imports)
        + """

pub fn parseStmt(sess) {
    if sess["pCur"].type == "EOF" {
        return null
    }
    sess["pIsPub"] = 0
    if sess["pCur"].type == TOKEN_PUB {
        bump(sess)
        sess["pIsPub"] = 1
    }
"""
        + "\n".join(dispatch)
        + """
    return parseStmt_expr(sess)
}
""",
    )


def main() -> None:
    split_postfix()
    split_compare()
    split_add_shift()
    split_stmt()
    print("done — run test_parser.kab and _parser_shard_times.py")


if __name__ == "__main__":
    main()
