#!/usr/bin/env python3
"""P6b: lift emit_impl module lets into session object E (same module first).

Does not split files yet — enables AccAdd-safe cross-shard extraction next.
"""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "self_host" / "emit_impl.kab"
SESSION = ROOT / "self_host" / "emit_session.kab"

text = SRC.read_text(encoding="utf-8")

# Collect `let eName = init` before first fn.
let_re = re.compile(r"^let (e[A-Za-z0-9]+) = (.+)$", re.M)
lets: list[tuple[str, str]] = []
for m in let_re.finditer(text):
    lets.append((m.group(1), m.group(2).rstrip()))

# Keep only those before first `fn ` / `pub fn `
first_fn = re.search(r"^(?:pub )?fn ", text, re.M)
assert first_fn, "no fn"
pre = text[: first_fn.start()]
lets = [(n, i) for n, i in lets if pre.find(f"let {n} =") >= 0]
names = [n for n, _ in lets]
print(f"session fields: {len(names)}")

# emit_session.kab
lines = [
    "// P6b: emit session object (pools/stacks/temps). Passed as E into shards.",
    "pub fn eMakeSession() {",
    "    return {",
]
for n, init in lets:
    lines.append(f'        "{n}": {init},')
lines.append("    }")
lines.append("}")
lines.append("")
lines.append("pub fn eResetSession(E) {")
for n, init in lets:
    lines.append(f'    E["{n}"] = {init}')
lines.append("}")
lines.append("")
SESSION.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
print("wrote", SESSION)

# Rewrite emit_impl: drop lets, import session, let E = eMakeSession(), replace names.
body = text[first_fn.start() :]
# Remove let block from header
header = """// H6e: self-hosted bytecode emitter body — heavy impl (skip-listed; see self_host/emit.kab).
// Produces opcode IR from AST.
// P6b: session object E holds pools/stacks (module lets are not shared across shards).
import \"self_host/ast_defs\"
import \"self_host/emit_defs\"
import \"self_host/emit_session\"

let E = eMakeSession()

"""

# Replace longest names first with E["name"]
# Avoid replacing inside strings roughly by doing word-boundary replace.
for n in sorted(names, key=len, reverse=True):
    body = re.sub(rf"\b{n}\b", f'E["{n}"]', body)

# emitImpl reset: was assigning each field; eResetSession can replace the big reset block.
# Keep assignments for now (already rewritten to E["…"] = …) — optionally later call eResetSession.

# Fix false positives: E["E"] if any? shouldn't exist.
# constKey and others OK.

out = header + body
# Comment that mentioned module-global — already new header.

SRC.write_text(out, encoding="utf-8", newline="\n")
print("rewrote", SRC, "lines", out.count(chr(10)) + 1)
