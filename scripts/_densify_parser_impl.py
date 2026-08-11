#!/usr/bin/env python3
"""P6b: densify parser_impl.kab into session + trampoline shards (emit-like).

Run from repo root:
  python scripts/_densify_parser_impl.py

Requires self_host/_parser_impl_pre_densify.kab (monolithic backup).
"""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SH = ROOT / "self_host"
SRC = SH / "_parser_impl_pre_densify.kab"

HOOKS = [
    ("parsePostfix", 0),
    ("parseTypeArgs", 1),
    ("parseUnary", 2),
    ("parseMul", 3),
    ("parseAddShift", 4),
    ("parseRelExpr", 5),
    ("parseCompare", 6),
    ("parseStmt", 7),
]
HOOK_ID = {fn: hid for fn, hid in HOOKS}

IMPORTS = """import "self_host/lexer_defs"
import "self_host/ast_defs"
"""


def write(name: str, text: str) -> None:
    p = SH / name
    p.write_text(text.lstrip("\n"), encoding="utf-8", newline="\n")
    print(f"wrote {p.relative_to(ROOT)} ({len(text.splitlines())} lines)")


def camel_to_snake(name: str) -> str:
    out: list[str] = []
    for i, ch in enumerate(name):
        if ch.isupper() and i > 0:
            out.append("_")
        out.append(ch.lower())
    return "".join(out)


def shard_name(fn_name: str) -> str:
    if fn_name in ("peek", "bump", "poolPush"):
        return "parser_util.kab"
    if fn_name == "parseTokensImpl":
        return "parser_main.kab"
    stem = fn_name[5:] if fn_name.startswith("parse") else fn_name
    return f"parser_{camel_to_snake(stem)}.kab"


def split_functions(text: str) -> dict[str, str]:
    first_fn = re.search(r"^(?:pub )?fn ", text, re.M)
    assert first_fn
    pre = text[: first_fn.start()]
    rest = text[first_fn.start() :]
    chunks = re.split(r"(?=^(?:pub )?fn \w+\()", rest, flags=re.M)
    fns: dict[str, str] = {}
    for chunk in chunks:
        chunk = chunk.lstrip("\n")
        if not chunk.strip():
            continue
        m = re.match(r"^(?:pub )?fn (\w+)", chunk)
        assert m, chunk[:80]
        fns[m.group(1)] = chunk
    return pre, fns


def session_from_lets(lets: list[tuple[str, str]]) -> str:
    lines = [
        "// P6b: parser session — thread sess into all shards.",
        "pub fn pMakeSession() {",
        "    let sess = {}",
    ]
    for n, init in lets:
        lines.append(f'    sess["{n}"] = {init}')
    lines += ["    return sess", "}", "", "pub fn pResetSession(sess) {"]
    for n, init in lets:
        lines.append(f'    sess["{n}"] = {init}')
    lines.append("}")
    lines.append("")
    return "\n".join(lines)


def add_sess_param(block: str, pub: bool = True) -> str:
    m = re.match(r"^(?:pub )?fn (\w+)\(([^)]*)\)", block.lstrip())
    if not m:
        return block
    fname, args = m.group(1), m.group(2).strip()
    if args == "sess" or args.startswith("sess,"):
        out = block
    else:
        new_args = "sess" if not args else f"sess, {args}"
        out = re.sub(
            rf"^(?:pub )?fn {fname}\({re.escape(args)}\)",
            f"{'pub fn' if pub else 'fn'} {fname}({new_args})",
            block.lstrip(),
            count=1,
        )
    if pub and not out.lstrip().startswith("pub fn "):
        out = re.sub(r"^fn ", "pub fn ", out.lstrip(), count=1)
    return out


def rewrite_calls(body: str) -> str:
    for fn in HOOK_ID:
        short = fn[5:]
        body = re.sub(rf"\b{fn}\(\)", f"pCall{short}(sess)", body)
    body = re.sub(r"\bpoolPush\(", "poolPush(sess, ", body)
    body = re.sub(r"\bpeek\(\)", "peek(sess)", body)
    body = re.sub(r"\bbump\(\)", "bump(sess)", body)
    return body


def rewrite_fields(body: str, names: list[str]) -> str:
    for n in sorted(names, key=len, reverse=True):
        body = re.sub(rf"\b{n}\b", f'sess["{n}"]', body)
    return body


def transform_block(block: str, names: list[str], pub: bool) -> str:
    block = add_sess_param(block, pub=pub)
    block = rewrite_fields(block, names)
    block = rewrite_calls(block)
    return block


POOL_PUSH_UTIL = """
// P6b: poolPush — AccAdd-safe (temp newPool; see emit eCallRet pattern).
pub fn poolPush(sess, v) {
    let newPool = push(sess["pSymPool"], v)
    sess["pSymPool"] = newPool
    sess["pSymN"] = sess["pSymN"] + 1
    return v
}
"""


def main() -> None:
    if not SRC.is_file():
        raise SystemExit(f"missing {SRC}; copy parser_impl.kab first")
    text = SRC.read_text(encoding="utf-8")
    pre, fns = split_functions(text)
    let_re = re.compile(r"^let (p[A-Za-z0-9]+) = (.+)$", re.M)
    lets = [(m.group(1), m.group(2).rstrip()) for m in let_re.finditer(pre)]
    names = [n for n, _ in lets]
    print(f"session fields={len(names)} functions={list(fns.keys())}")

    write("parser_session.kab", session_from_lets(lets))

    hook_lines = ["// P6b: parser trampoline hooks.", ""]
    for fn, hid in HOOKS:
        short = fn[5:]
        hook_lines += [
            f"pub fn pCall{short}(sess) {{",
            "    let prevH = sess[\"_hook\"]",
            f"    sess[\"_hook\"] = {hid}",
            "    let r = sess[\"tramp\"]()",
            "    sess[\"_hook\"] = prevH",
            "    return r",
            "}",
            "",
        ]
    write("parser_hooks.kab", "\n".join(hook_lines))

    tramp_imports: list[str] = []
    tramp_body = ["pub fn pTramp(sess) {"]
    util_parts: list[str] = []

    for fn_name, block in fns.items():
        if fn_name == "parseTokensImpl":
            continue
        block = transform_block(block, names, pub=True)
        if fn_name in ("peek", "bump"):
            util_parts.append(block)
            continue
        if fn_name == "poolPush":
            continue
        shard = shard_name(fn_name)
        tramp_imports.append(f'import "self_host/{shard[:-4]}"')
        tramp_body.append(f'    if sess["_hook"] == {HOOK_ID[fn_name]} {{')
        tramp_body.append(f"        return {fn_name}(sess)")
        tramp_body.append("    }")
        write(
            shard,
            IMPORTS
            + 'import "self_host/parser_hooks"\nimport "self_host/parser_util"\n\n'
            + block
            + "\n",
        )

    tramp_body.append('    throw "parser tramp: bad hook " + ("" + sess["_hook"])')
    tramp_body.append("}")
    write("parser_tramp.kab", "\n".join(tramp_imports) + "\n\n" + "\n".join(tramp_body) + "\n")
    util_text = IMPORTS + "\n" + "\n".join(util_parts) + POOL_PUSH_UTIL
    write("parser_util.kab", util_text)

    main_block = transform_block(fns["parseTokensImpl"], names, pub=True)
    main_block = main_block.replace("pub fn parseTokensImpl(sess, tokens)", "pub fn parseMain(sess, tokens)")
    write(
        "parser_main.kab",
        IMPORTS
        + 'import "self_host/parser_hooks"\nimport "self_host/parser_util"\n\n'
        + main_block
        + "\n",
    )

    write(
        "parser_exec.kab",
        """// P6b: module session + trampoline entry.
import "self_host/parser_session"
import "self_host/parser_tramp"
import "self_host/parser_main"

let sess = pMakeSession()

fn tramp() {
    return pTramp(sess)
}

sess["tramp"] = tramp
sess["_hook"] = 0

pub fn parseTokensExec(tokens) {
    pResetSession(sess)
    sess["tramp"] = tramp
    sess["_hook"] = 0
    return parseMain(sess, tokens)
}
""",
    )

    write(
        "parser_impl.kab",
        """// H6e: thin parser driver (skip-listed leaf).
import "self_host/parser_exec"

pub fn parseTokensImpl(tokens) {
    return parseTokensExec(tokens)
}
""",
    )

    print("done — run test_parser.kab")


if __name__ == "__main__":
    main()
