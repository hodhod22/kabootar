#!/usr/bin/env python3
"""Fix E0308/E0596/E0277/E0614 on lines cargo points to."""

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


def cargo_errors() -> list[dict]:
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
        if not raw.strip().startswith("{"):
            continue
        try:
            o = json.loads(raw)
        except json.JSONDecodeError:
            continue
        if o.get("reason") != "compiler-message":
            continue
        msg = o.get("message", {})
        if msg.get("level") != "error":
            continue
        out.append(msg)
    return out


def path_of(sp: dict) -> Path | None:
    fn = sp.get("file_name")
    if not fn:
        return None
    p = Path(fn)
    return p if p.is_absolute() else ROOT / p


def fix_e0308(line: str, msg: str) -> str:
    new = line
    if "expected struct `Rc" in msg or "expected `Rc" in msg:
        if "Value::Array(" in new and "from_array" not in new:
            new = new.replace("Value::Array(", "Value::from_array(", 1)
        if "Value::Object(" in new and "from_object" not in new:
            new = new.replace("Value::Object(", "Value::from_object(", 1)
    if "expected struct `Vec" in msg or "expected `Vec" in msg:
        new = re.sub(r"Value::from_array\(", "Value::Array(", new, count=1)
        new = re.sub(r"Value::from_object\(", "Value::Object(", new, count=1)
    if "expected `&mut Rc" in msg and "Rc::make_mut" not in new:
        # wrong make_mut target — if pattern binds ref mut o, keep; else fix HashMap direct
        pass
    return new


def fix_e0596(line: str) -> str:
    new = line
    for m in re.finditer(r"Value::Array\(ref mut ([a-zA-Z_]\w*)\)", line):
        name = m.group(1)
        if f"Rc::make_mut({name})" in new:
            continue
        new = re.sub(rf"\b{re.escape(name)}\.", f"Rc::make_mut({name}).", new)
        new = re.sub(rf"\b{re.escape(name)}\[", f"Rc::make_mut({name})[", new)
    for m in re.finditer(r"Value::Object\(ref mut ([a-zA-Z_]\w*)\)", line):
        name = m.group(1)
        if f"Rc::make_mut({name})" in new:
            continue
        new = re.sub(rf"\b{re.escape(name)}\.", f"Rc::make_mut({name}).", new)
    # Value::Array(items) without ref mut in match - add ref mut on prior line handled separately
    return new


def fix_e0277(line: str, msg: str) -> str:
    if "is not an iterator" in msg:
        line = re.sub(
            r"\bfor ([a-zA-Z_]\w*) in ([a-zA-Z_]\w*)\s*\{",
            r"for \1 in \2.iter() {",
            line,
        )
        line = re.sub(
            r"\bfor \(([a-zA-Z_]\w*),\s*([a-zA-Z_]\w*)\) in ([a-zA-Z_]\w*)\s*\{",
            r"for (\1, \2) in \3.iter() {",
            line,
        )
    if "cannot be built from an iterator" in msg and ".collect()" in line:
        line = re.sub(
            r"Value::Array\(([^)]+\.collect[^)]*)\)",
            r"Value::from_array(\1)",
            line,
        )
        line = re.sub(
            r"Value::Object\(([^)]+\.collect[^)]*)\)",
            r"Value::from_object(\1)",
            line,
        )
    return line


def fix_e0614(line: str) -> str:
    return re.sub(r"(?<![=<>!])\*([a-zA-Z_]\w*)", r"\1", line)


def apply_once(msgs: list[dict]) -> int:
    file_lines: dict[Path, dict[int, list[tuple[str, str]]]] = {}
    for msg in msgs:
        code = (msg.get("code") or {}).get("code")
        text = msg.get("message", "")
        sp = (msg.get("spans") or [{}])[0]
        path = path_of(sp)
        if path is None or not path.exists():
            continue
        line_no = sp.get("line_start", 0)
        if line_no <= 0:
            continue
        file_lines.setdefault(path, {}).setdefault(line_no, []).append((code or "", text))

    changes = 0
    for path, line_map in file_lines.items():
        lines = path.read_text(encoding="utf-8").splitlines()
        for line_no, hints in line_map.items():
            idx = line_no - 1
            if idx >= len(lines):
                continue
            old = lines[idx]
            new = old
            for code, text in hints:
                if code == "E0308":
                    new = fix_e0308(new, text)
                elif code == "E0596":
                    new = fix_e0596(new)
                elif code == "E0277":
                    new = fix_e0277(new, text)
                elif code == "E0614":
                    new = fix_e0614(new)
            if new != old:
                lines[idx] = new
                changes += 1
        content = "\n".join(lines)
        if path.read_text(encoding="utf-8").endswith("\n"):
            content += "\n"
        path.write_text(content, encoding="utf-8")
    return changes


def main() -> None:
    for i in range(15):
        msgs = cargo_errors()
        counts: dict[str, int] = {}
        for m in msgs:
            c = (m.get("code") or {}).get("code", "?")
            counts[c] = counts.get(c, 0) + 1
        total = len(msgs)
        if total == 0:
            print(f"iter {i+1}: SUCCESS")
            return
        n = apply_once(msgs)
        print(f"iter {i+1}: fixes={n} errors={total} {dict(sorted(counts.items()))}")
        if n == 0:
            break


if __name__ == "__main__":
    main()
