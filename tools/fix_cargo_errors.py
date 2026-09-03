#!/usr/bin/env python3
"""Apply targeted fixes from cargo check JSON errors."""

from __future__ import annotations

import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "src"
ENV = {
    **dict(__import__("os").environ),
    "CARGO_TARGET_DIR": str(ROOT / "target-p6b9"),
    "PATH": "/c/Users/hodho/.cargo/bin:" + __import__("os").environ.get("PATH", ""),
}


def cargo_json() -> list[dict]:
    proc = subprocess.run(
        ["cargo", "check", "--message-format=json"],
        cwd=ROOT,
        env=ENV,
        capture_output=True,
        text=True,
        errors="replace",
    )
    out = []
    for line in proc.stdout.splitlines():
        if not line.strip().startswith("{"):
            continue
        try:
            o = json.loads(line)
        except json.JSONDecodeError:
            continue
        if o.get("reason") != "compiler-message":
            continue
        m = o.get("message", {})
        if m.get("level") != "error":
            continue
        out.append(m)
    return out


def rel_path(file_name: str) -> Path:
    p = Path(file_name)
    if p.is_absolute():
        return p
    return ROOT / p


def fix_line_e0164(line: str) -> str:
    new = line
    subs = [
        (r"Value::from_array\(", "Value::Array("),
        (r"Value::from_object\(", "Value::Object("),
    ]
    for pat, repl in subs:
        new = re.sub(pat, repl, new)
    return new


def fix_line_e0308_rc(line: str, msg: str) -> str:
    new = line
    if "expected struct `Rc" in msg or "expected `Rc" in msg:
        if "Value::Array(" in new and "Value::from_array(" not in new:
            new = new.replace("Value::Array(", "Value::from_array(", 1)
        if "Value::Object(" in new and "Value::from_object(" not in new:
            new = new.replace("Value::Object(", "Value::from_object(", 1)
    if "expected struct `Vec" in msg or "expected `Vec" in msg:
        # from_array got Rc instead of Vec
        new = re.sub(
            r"Value::from_array\(([^)]+\.clone\(\))\)",
            r"Value::Array(\1)",
            new,
        )
        new = re.sub(
            r"Value::from_object\(([^)]+\.clone\(\))\)",
            r"Value::Object(\1)",
            new,
        )
    return new


def fix_line_e0277(line: str, msg: str) -> str:
    if "is not an iterator" not in msg:
        return line
    # for x in items { -> for x in items.iter() {
    new = re.sub(
        r"\bfor ([a-zA-Z_][a-zA-Z0-9_]*) in ([a-zA-Z_][a-zA-Z0-9_]*)\s*\{",
        r"for \1 in \2.iter() {",
        line,
    )
    new = re.sub(
        r"\bfor \(([a-zA-Z_][a-zA-Z0-9_]*),\s*([a-zA-Z_][a-zA-Z0-9_]*)\) in ([a-zA-Z_][a-zA-Z0-9_]*)\s*\{",
        r"for (\1, \2) in \3.iter() {",
        new,
    )
    if ".collect()" in line and "Value::from_array" not in line:
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
    return new


def fix_line_e0596(line: str) -> str:
    # ref mut binding + mutation
    m = re.search(r"Value::Array\(ref mut ([a-zA-Z_][a-zA-Z0-9_]*)\)", line)
    if m:
        name = m.group(1)
        if f"Rc::make_mut({name})" not in line:
            line = re.sub(rf"\b{re.escape(name)}\.", f"Rc::make_mut({name}).", line)
            line = re.sub(
                rf"\b{re.escape(name)}\[",
                f"Rc::make_mut({name})[",
                line,
            )
    m = re.search(r"Value::Object\(ref mut ([a-zA-Z_][a-zA-Z0-9_]*)\)", line)
    if m:
        name = m.group(1)
        if f"Rc::make_mut({name})" not in line:
            line = re.sub(rf"\b{re.escape(name)}\.", f"Rc::make_mut({name}).", line)
    return line


def fix_line_e0614(line: str) -> str:
    # remove spurious * on scalar bindings
    return re.sub(r"\*([a-zA-Z_][a-zA-Z0-9_]*)", r"\1", line)


def apply_fixes(msgs: list[dict]) -> int:
    file_lines: dict[Path, dict[int, list[str]]] = {}
    for msg in msgs:
        code = (msg.get("code") or {}).get("code")
        text = msg.get("message", "")
        sp = (msg.get("spans") or [{}])[0]
        fn = sp.get("file_name")
        if not fn or not str(fn).endswith(".rs"):
            continue
        path = rel_path(fn)
        if not path.exists():
            continue
        line_no = sp.get("line_start", 0)
        if line_no <= 0:
            continue
        file_lines.setdefault(path, {}).setdefault(line_no, []).append(code or "")

    changes = 0
    for path, line_map in file_lines.items():
        lines = path.read_text(encoding="utf-8").splitlines()
        for line_no, codes in line_map.items():
            idx = line_no - 1
            if idx >= len(lines):
                continue
            old = lines[idx]
            new = old
            for msg in msgs:
                sp = (msg.get("spans") or [{}])[0]
                if rel_path(sp.get("file_name", "")) != path:
                    continue
                if sp.get("line_start") != line_no:
                    continue
                code = (msg.get("code") or {}).get("code")
                text = msg.get("message", "")
                if code == "E0164":
                    new = fix_line_e0164(new)
                elif code == "E0308":
                    new = fix_line_e0308_rc(new, text)
                elif code == "E0277":
                    new = fix_line_e0277(new, text)
                elif code == "E0596":
                    new = fix_line_e0596(new)
                elif code == "E0614":
                    new = fix_line_e0614(new)
            if new != old:
                lines[idx] = new
                changes += 1
        content = "\n".join(lines)
        if path.read_text(encoding="utf-8").endswith("\n"):
            content += "\n"
        path.write_text(content, encoding="utf-8")
    return changes


def count_errors() -> tuple[int, dict[str, int]]:
    msgs = cargo_json()
    counts: dict[str, int] = {}
    for msg in msgs:
        code = (msg.get("code") or {}).get("code", "?")
        counts[code] = counts.get(code, 0) + 1
    return len(msgs), counts


def main() -> None:
    for i in range(12):
        msgs = cargo_json()
        if not msgs:
            print(f"iter {i+1}: SUCCESS")
            return
        n = apply_fixes(msgs)
        total, counts = count_errors()
        print(f"iter {i+1}: line_fixes={n} errors={total} {dict(sorted(counts.items()))}")
        if total == 0:
            print("SUCCESS")
            return
        if n == 0:
            print("stalled")
            break


if __name__ == "__main__":
    main()
