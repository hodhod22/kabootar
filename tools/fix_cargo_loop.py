#!/usr/bin/env python3
"""Aggressive line-level fixes from cargo JSON."""

from __future__ import annotations

import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ENV = {
    **dict(__import__("os").environ),
    "CARGO_TARGET_DIR": str(ROOT / "target-p6b9"),
    "PATH": "/c/Users/hodho/.cargo/bin:" + __import__("os").environ.get("PATH", ""),
}


def cargo_msgs() -> list[dict]:
    proc = subprocess.run(
        ["cargo", "check", "--message-format=json"],
        cwd=ROOT,
        env=ENV,
        capture_output=True,
        text=True,
        errors="replace",
    )
    out = []
    for raw in proc.stdout.splitlines():
        try:
            o = json.loads(raw)
        except json.JSONDecodeError:
            continue
        if o.get("reason") == "compiler-message" and o.get("message", {}).get("level") == "error":
            out.append(o["message"])
    return out


def rel(path: str) -> Path:
    p = Path(path)
    return p if p.is_absolute() else ROOT / p


def fix_line(line: str, code: str, msg: str) -> str:
    new = line
    m = msg.lower()
    if code == "E0308":
        if "expected struct `rc" in m or "expected `rc" in m:
            if "Value::Array(" in new:
                new = new.replace("Value::Array(", "Value::from_array(", 1)
            if "Value::Object(" in new:
                new = new.replace("Value::Object(", "Value::from_object(", 1)
        if "expected struct `vec" in m or "expected `vec" in m:
            new = re.sub(r"Value::from_array\(", "Value::Array(", new, count=1)
            new = re.sub(r"Value::from_object\(", "Value::Object(", new, count=1)
        if "expected `&i64`" in m and "*n" not in new and re.search(r"\bn\b", new):
            new = re.sub(r"\bn\b", "*n", new)
        if "expected `&mut rc" in m and "ref mut" not in new and "Value::Object(map)" in new:
            new = new.replace("Value::Object(map)", "Value::Object(ref mut map)")
        if "found `hashmap" in m and "rc::make_mut" in new.lower():
            # Rc::make_mut on bare HashMap var — replace with direct call
            new = re.sub(r"Rc::make_mut\(([a-zA-Z_]\w*)\)\.", r"\1.", new)
    elif code == "E0596":
        if "cannot borrow data in an `rc`" in m.lower():
            # insert Rc::make_mut on first identifier before .method
            new = re.sub(
                r"(\b[a-zA-Z_]\w*)\.(push|pop|insert|remove|extend|clear)\(",
                r"Rc::make_mut(\1).\2(",
                new,
                count=1,
            )
        if "ref mut items" in new and "Rc::make_mut(items)" not in new:
            new = re.sub(
                r"\bitems\.(push|pop|insert|remove|extend)\(",
                r"Rc::make_mut(items).\1(",
                new,
            )
        if "not declared as mutable" in m.lower() and "let Value::Array(ref mut" in new:
            new = new.replace(
                "let Value::Array(ref mut items) = arr",
                "let Value::Array(ref mut items) = &mut arr",
            )
    elif code == "E0277":
        if "is not an iterator" in m.lower():
            new = re.sub(
                r"\bfor ([a-zA-Z_]\w*) in ([a-zA-Z_]\w*)\s*\{",
                r"for \1 in \2.iter() {",
                new,
            )
        if "cannot be built from an iterator" in m.lower():
            new = re.sub(
                r"Value::Array\(([^)]+\.collect[^)]*)\)",
                r"Value::from_array(\1)",
                new,
            )
            new = re.sub(
                r"Value::Object\(([^)]+\.collect[^)]*)\)",
                r"Value::from_object(\1)",
                new,
            )
    elif code == "E0614":
        new = re.sub(r"(?<![=<>!])\*([a-zA-Z_]\w*)", r"\1", new)
    elif code == "E0606":
        new = re.sub(r"\*([a-zA-Z_]\w*) as usize", r"\1 as usize", new)
    return new


def apply(msgs: list[dict]) -> int:
    edits: dict[Path, dict[int, list[tuple[str, str]]]] = {}
    for msg in msgs:
        code = (msg.get("code") or {}).get("code", "")
        text = msg.get("message", "")
        sp = (msg.get("spans") or [{}])[0]
        fn = sp.get("file_name")
        if not fn:
            continue
        path = rel(fn)
        ln = sp.get("line_start", 0)
        if ln <= 0:
            continue
        edits.setdefault(path, {}).setdefault(ln, []).append((code, text))

    changes = 0
    for path, lmap in edits.items():
        if not path.exists():
            continue
        lines = path.read_text(encoding="utf-8").splitlines()
        for ln, hints in lmap.items():
            i = ln - 1
            if i >= len(lines):
                continue
            old = lines[i]
            new = old
            for code, text in hints:
                new = fix_line(new, code, text)
            if new != old:
                lines[i] = new
                changes += 1
        content = "\n".join(lines)
        if path.read_text(encoding="utf-8").endswith("\n"):
            content += "\n"
        path.write_text(content, encoding="utf-8")
    return changes


def main() -> None:
    for i in range(20):
        msgs = cargo_msgs()
        counts: dict[str, int] = {}
        for m in msgs:
            c = (m.get("code") or {}).get("code", "?")
            counts[c] = counts.get(c, 0) + 1
        total = len(msgs)
        if total == 0:
            print("SUCCESS")
            return
        n = apply(msgs)
        print(f"iter {i+1}: fixes={n} errors={total} {dict(sorted(counts.items()))}")
        if n == 0:
            break


if __name__ == "__main__":
    main()
