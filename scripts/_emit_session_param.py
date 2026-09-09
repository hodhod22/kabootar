#!/usr/bin/env python3
"""P6b: emit densify — session E as *parameter* (not module let).

Produces:
  emit_session.kab  — eMakeSession / eResetSession
  emit_main.kab     — full emitter taking E as first arg
  emit_impl.kab     — thin: make session + emitMain(E, program)

Also rewrites `E[k] = symIndex(E,...)` (etc.) to `eCallRet = …; E[k] = eCallRet`
because host AccAdd of `E[k]=fn(E,…)` can pass a clone into fn (pools lost).
"""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "self_host" / "emit_impl.kab"
SESSION = ROOT / "self_host" / "emit_session.kab"
MAIN = ROOT / "self_host" / "emit_main.kab"
IMPL = ROOT / "self_host" / "emit_impl.kab"

text = SRC.read_text(encoding="utf-8")
if text.count("\n") < 500 or "let eConsts" not in text:
    raise SystemExit(
        f"{SRC} looks thin/already converted; restore full emit_impl from git first "
        "(e.g. git show HEAD:self_host/emit_impl.kab)"
    )
# Self-host parser: binary ops inside object-literal fields → "invalid assign lhs".
text = text.replace(
    """        let ai = len(eArrows)
        eArrows = push(eArrows, {
            \"sym\": \"__arrow_\" + ai,""",
    """        let ai = len(eArrows)
        let arrowSym = \"__arrow_\" + ai
        eArrows = push(eArrows, {
            \"sym\": arrowSym,""",
    1,
)
let_re = re.compile(r"^let (e[A-Za-z0-9]+) = (.+)$", re.M)
first_fn = re.search(r"^(?:pub )?fn ", text, re.M)
assert first_fn
pre = text[: first_fn.start()]
lets = [(m.group(1), m.group(2).rstrip()) for m in let_re.finditer(pre)]
names = [n for n, _ in lets]
print(f"fields={len(names)}")

sl = [
    "// P6b: emit session (pools/stacks/temps). Thread as E into shards.",
    "// Imperative init — huge object literals are slow for self-host compile.",
    "pub fn eMakeSession() {",
    "    let E = {}",
]
for n, init in lets:
    sl.append(f'    E["{n}"] = {init}')
sl += [
    "    return E",
    "}",
    "",
    "pub fn eResetSession(E) {",
]
for n, init in lets:
    sl.append(f'    E["{n}"] = {init}')
sl.append("}")
sl.append("")
SESSION.write_text("\n".join(sl) + "\n", encoding="utf-8", newline="\n")

body = text[first_fn.start() :]

for n in sorted(names, key=len, reverse=True):
    body = re.sub(rf"\b{n}\b", f'E["{n}"]', body)


def add_e_param(m: re.Match) -> str:
    prefix = m.group(1) or ""
    name = m.group(2)
    args = m.group(3)
    if name == "emitImpl":
        name = "emitMain"
    if args.strip() == "":
        return f"{prefix}fn {name}(E)"
    return f"{prefix}fn {name}(E, {args})"


body = re.sub(r"^(pub )?fn ([A-Za-z0-9_]+)\(([^)]*)\)", add_e_param, body, flags=re.M)

FNS = [
    "constKey",
    "resetLocalMap",
    "rebuildLocalMap",
    "symIndex",
    "arrUtil",
    "emitOp",
    "emitCallCallee",
    "dropCallCallee",
    "emitCallArgExprs",
    "emitSym",
    "localSymIndex",
    "tryEmitArrayPushAssign",
    "tryEmitLenCall",
    "tryEmitAccAddAssign",
    "emitExpr",
    "patchRelJump",
    "pushJmpAtLen",
    "emitIfStmt",
    "emitStmt",
    "emitMain",
]


def thread_calls(src: str) -> str:
    for fn in sorted(FNS, key=len, reverse=True):
        src = re.sub(rf"\b{fn}\(\)", f"{fn}(E)", src)
        # Thread E unless already fn(E) or fn(E, ...)
        src = re.sub(rf"\b{fn}\((?!E[,)])", f"{fn}(E, ", src)
    return src


body = thread_calls(body)
body = body.replace("\neLetPub = ", "\n        let eLetPub = ")

# Host AccAdd bug: `E["k"] = fn(E, ...)` may pass a *clone* of E into fn, so
# side effects (const/global pools) are lost while the return value still lands
# on the real E. Rewrite those sites via module scratch `eCallRet`.
# (Do not use many `let` temps inside emitExpr — locals get unreliable there.)
ACCADD_SIDE_FNS = [
    "symIndex",
    "localSymIndex",
    "patchRelJump",
    "pushJmpAtLen",
]
_acc_alt = "|".join(sorted(ACCADD_SIDE_FNS, key=len, reverse=True))
_acc_pat = re.compile(
    rf'^(\s*)E\["([^"]+)"\]\s*=\s*({_acc_alt})\(E\b([^;\n]*)$',
    re.M,
)


def rewrite_accadd_call_e(src: str) -> str:
    out: list[str] = []
    i = 0
    n = 0
    for m in _acc_pat.finditer(src):
        out.append(src[i : m.start()])
        indent, field, fn, rest = m.group(1), m.group(2), m.group(3), m.group(4)
        n += 1
        out.append(f"{indent}eCallRet = {fn}(E{rest}\n")
        out.append(f'{indent}E["{field}"] = eCallRet')
        i = m.end()
    out.append(src[i:])
    print(f"accadd-sideeffect rewrites={n}")
    text2 = "".join(out)
    # Prefer eCallRet for the common emitOp(E, OP_*, E["eIdx"]) follow-up.
    text2, c = re.subn(
        r'(eCallRet = symIndex\(E[^\n]*\n)'
        r'(\s*)E\["eIdx"\] = eCallRet\n'
        r'(\s*)emitOp\(E, ([A-Z0-9_]+), E\["eIdx"\]\)',
        r'\1\2E["eIdx"] = eCallRet\n\3emitOp(E, \4, eCallRet)',
        text2,
    )
    print(f"emitOp eCallRet fuses={c}")
    return text2


body = rewrite_accadd_call_e(body)


def rewrite_chained_member_plus(src: str) -> str:
    """Self-host parser rejects `E[k] = E[k] + a + b` (invalid assign lhs)."""
    pat = re.compile(
        r'^(\s*)E\["(eMangled|eMethodMangled)"\] = E\["\2"\] \+ ("[^"]+") \+ (.+)$',
        re.M,
    )

    def repl(m: re.Match) -> str:
        ind, field, sep, rhs = m.group(1), m.group(2), m.group(3), m.group(4)
        return (
            f'{ind}eCallRet = E["{field}"] + {sep}\n'
            f'{ind}E["{field}"] = eCallRet + {rhs}'
        )

    text2, n = pat.subn(repl, src)
    print(f"chained-member-plus rewrites={n}")
    return text2


body = rewrite_chained_member_plus(body)


def rewrite_op_patch_binop_args(src: str) -> str:
    """Self-host parser rejects binary ops inside object-literal fields."""
    pat = re.compile(
        r'^(\s*)(E\["e(?:Fn)?Ops"\]\[[^\]]+\]) = \{ "op": ([A-Z0-9_]+), "arg": ([^}]+) \}$',
        re.M,
    )
    n = 0
    out: list[str] = []
    i = 0
    for m in pat.finditer(src):
        arg = m.group(4).strip()
        if not re.search(r"[+\-*/]", arg):
            continue
        if re.fullmatch(r'E\["[^"]+"\]', arg) or re.fullmatch(
            r"[A-Za-z_][A-Za-z0-9_]*", arg
        ):
            continue
        n += 1
        ind, lhs, op = m.group(1), m.group(2), m.group(3)
        out.append(src[i : m.start()])
        out.append(
            f"{ind}eCallRet = {arg}\n{ind}{lhs} = {{ \"op\": {op}, \"arg\": eCallRet }}"
        )
        i = m.end()
    out.append(src[i:])
    print(f"op-patch binop-arg rewrites={n}")
    return "".join(out)


body = rewrite_op_patch_binop_args(body)


def rewrite_chained_member_sub(src: str) -> str:
    """Self-host parser rejects `E[k] = a - b - c` (invalid assign lhs)."""
    pat = re.compile(
        r'^(\s*)E\["([^"]+)"\] = (.+?) - (.+?) - (.+)$',
        re.M,
    )

    def repl(m: re.Match) -> str:
        ind, field, a, b, c = m.group(1), m.group(2), m.group(3), m.group(4), m.group(5)
        return (
            f"{ind}eCallRet = {a} - {b}\n"
            f'{ind}E["{field}"] = eCallRet - {c}'
        )

    text2, n = pat.subn(repl, src)
    print(f"chained-member-sub rewrites={n}")
    return text2


body = rewrite_chained_member_sub(body)


def rewrite_len_minus_one_peek(src: str) -> str:
    """Self-host parser rejects `E[dst] = E[stack][len(E[stack]) - 1]`."""
    pat = re.compile(
        r'^(\s*)E\["([^"]+)"\] = E\["([^"]+)"\]\[len\(E\["\3"\]\) - 1\]\s*$',
        re.M,
    )

    def repl(m: re.Match) -> str:
        ind, dst, stack = m.group(1), m.group(2), m.group(3)
        return (
            f'{ind}eCallRet = len(E["{stack}"]) - 1\n'
            f'{ind}E["{dst}"] = E["{stack}"][eCallRet]'
        )

    text2, n = pat.subn(repl, src)
    print(f"len-1 peek rewrites={n}")
    return text2


body = rewrite_len_minus_one_peek(body)


def rewrite_exports_concat_push(src: str) -> str:
    """Self-host parser rejects chained `+` inside some call args used in AccAdd."""
    pat = re.compile(
        r'^(\s*)E\["eExports"\] = push\(E\["eExports"\], \("" \+ E\["([^"]+)"\]\) \+ ""\)$',
        re.M,
    )

    def repl(m: re.Match) -> str:
        ind, field = m.group(1), m.group(2)
        return (
            f'{ind}eCallRet = ("" + E["{field}"]) + ""\n'
            f'{ind}E["eExports"] = push(E["eExports"], eCallRet)'
        )

    text2, n = pat.subn(repl, src)
    print(f"exports-concat-push rewrites={n}")
    return text2


body = rewrite_exports_concat_push(body)

# Full-source patches sometimes introduce `let __m` for chained +; emitExpr
# cannot reliably use many block lets — fold onto eCallRet.
body, n_m = re.subn(r"\blet __m = ", "eCallRet = ", body)
body = body.replace("= __m +", "= eCallRet +")
print(f"let-__m-to-eCallRet rewrites={n_m}")

main = f"""// P6b: emit body — session E threaded as first arg (AccAdd-safe across shards).
// Host: never AccAdd E[k]=fn(E,...) for pool side effects (clone); use eCallRet.
import \"self_host/ast_defs\"
import \"self_host/emit_defs\"

let eCallRet = 0
let eCallRet2 = 0

{body}
"""
MAIN.write_text(main, encoding="utf-8", newline="\n")
print("wrote", MAIN, "lines", main.count("\n") + 1)

impl = """// H6e: self-hosted bytecode emitter body — thin session driver (skip-listed).
// Heavy logic in emit_main.kab; E is a parameter there (not a module let).
import \"self_host/emit_session\"
import \"self_host/emit_main\"

pub fn emitImpl(program) {
    let E = eMakeSession()
    return emitMain(E, program)
}
"""
IMPL.write_text(impl, encoding="utf-8", newline="\n")
print("wrote", IMPL)
