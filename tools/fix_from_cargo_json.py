#!/usr/bin/env python3
"""Apply cargo JSON E0308 line fixes for Rc migration."""

from __future__ import annotations

import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def cargo_errors():
    env = {
        **dict(subprocess.os.environ),
        "CARGO_TARGET_DIR": str(ROOT / "target-p6b9"),
        "PATH": "/c/Users/hodho/.cargo/bin:" + subprocess.os.environ.get("PATH", ""),
    }
    proc = subprocess.run(
        ["cargo", "check", "--message-format=json"],
        cwd=ROOT,
        env=env,
        capture_output=True,
        text=True,
        errors="replace",
    )
    fixes: dict[str, dict[int, set[str]]] = {}
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
        code = m.get("code", {}).get("code")
        msg = m.get("message", "")
        sp = (m.get("spans") or [{}])[0]
        f = sp.get("file_name", "")
        ln = sp.get("line_start", 0)
        if not f.endswith(".rs") or ln <= 0:
            continue
        path = str(Path(f).resolve())
        fixes.setdefault(path, {}).setdefault(ln, set())
        if code == "E0308" and "expected struct `Rc" in msg:
            fixes[path][ln].add("wrap_ctor")
        elif code == "E0308" and "expected struct `Vec" in msg:
            fixes[path][ln].add("to_vec")
        elif code == "E0596":
            fixes[path][ln].add("make_mut")
    return fixes


def apply_line_fix(line: str, kinds: set[str]) -> str:
    new = line
    if "wrap_ctor" in kinds:
        if "serde_json::" not in new and "crate::value::" not in new.split("Value::")[0][-20:]:
            new = re.sub(r"\bValue::Array\(", "Value::from_array(", new)
            new = re.sub(r"\bValue::Object\(", "Value::from_object(", new)
    if "to_vec" in kinds:
        if ".to_vec()" not in new:
            new = re.sub(
                r"(\bcall_value\([^,]+,\s*)([a-zA-Z_][a-zA-Z0-9_]*)(\s*,)",
                r"\1\2.to_vec()\3",
                new,
            )
            new = re.sub(
                r"(\binstantiate_class\([^,]+,\s*[^,]+,\s*)([a-zA-Z_][a-zA-Z0-9_]*)(\s*,)",
                r"\1\2.to_vec()\3",
                new,
            )
    return new


def main() -> None:
    fixes = cargo_errors()
    total = 0
    for path_str, line_fixes in fixes.items():
        path = Path(path_str)
        if not path.exists() or "target" in path.parts:
            continue
        lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
        changed = False
        for ln, kinds in line_fixes.items():
            if ln > len(lines):
                continue
            idx = ln - 1
            old = lines[idx]
            new = apply_line_fix(old, kinds)
            if new != old:
                lines[idx] = new if old.endswith("\n") and not new.endswith("\n") else new
                if not lines[idx].endswith("\n") and old.endswith("\n"):
                    lines[idx] += "\n"
                changed = True
                total += 1
        if changed:
            path.write_text("".join(lines), encoding="utf-8")
    print(f"line_fixes={total}")


if __name__ == "__main__":
    main()
