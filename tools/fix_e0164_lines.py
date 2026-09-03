#!/usr/bin/env python3
"""Fix E0164 lines reported by cargo — revert from_* to enum variants on those lines."""

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


def fix_line(line: str) -> str:
    line = line.replace("Value::from_array(", "Value::Array(")
    line = line.replace("Value::from_object(", "Value::Object(")
    line = line.replace("crate::value::Value::from_array(", "crate::value::Value::Array(")
    line = line.replace("crate::value::Value::from_object(", "crate::value::Value::Object(")
    return line


def main() -> None:
    proc = subprocess.run(
        ["cargo", "check", "--message-format=json"],
        cwd=ROOT,
        env=ENV,
        capture_output=True,
        text=True,
        errors="replace",
    )
    edits: dict[Path, set[int]] = {}
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
        if (msg.get("code") or {}).get("code") != "E0164":
            continue
        sp = (msg.get("spans") or [{}])[0]
        fn = sp.get("file_name")
        if not fn:
            continue
        path = Path(fn)
        if not path.is_absolute():
            path = ROOT / path
        line_no = sp.get("line_start", 0)
        if line_no > 0:
            edits.setdefault(path, set()).add(line_no)

    changes = 0
    for path, lines_set in edits.items():
        if not path.exists():
            continue
        lines = path.read_text(encoding="utf-8").splitlines()
        for line_no in lines_set:
            idx = line_no - 1
            if idx >= len(lines):
                continue
            new = fix_line(lines[idx])
            if new != lines[idx]:
                lines[idx] = new
                changes += 1
        content = "\n".join(lines)
        if path.read_text(encoding="utf-8").endswith("\n"):
            content += "\n"
        path.write_text(content, encoding="utf-8")
    print(f"fixed {changes} lines in {len(edits)} files")


if __name__ == "__main__":
    main()
