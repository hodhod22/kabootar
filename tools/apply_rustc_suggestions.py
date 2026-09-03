#!/usr/bin/env python3
"""Apply MachineApplicable rustc suggestions from cargo check JSON."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ENV = {
    **dict(__import__("os").environ),
    "CARGO_TARGET_DIR": str(ROOT / "target-p6b9"),
    "PATH": "/c/Users/hodho/.cargo/bin:" + __import__("os").environ.get("PATH", ""),
}


def collect_suggestions() -> dict[Path, list[tuple[int, int, int, str]]]:
    proc = subprocess.run(
        ["cargo", "check", "--message-format=json"],
        cwd=ROOT,
        env=ENV,
        capture_output=True,
        text=True,
        errors="replace",
    )
    # (line, col_start, col_end, replacement) — apply in reverse order per file
    edits: dict[Path, list[tuple[int, int, int, str]]] = {}
    for raw in proc.stdout.splitlines():
        try:
            o = json.loads(raw)
        except json.JSONDecodeError:
            continue
        if o.get("reason") != "compiler-message":
            continue
        msg = o.get("message", {})
        if msg.get("level") != "error":
            continue

        def walk(m: dict) -> None:
            for child in m.get("children") or []:
                for sp in child.get("spans") or []:
                    rep = sp.get("suggested_replacement")
                    app = sp.get("suggestion_applicability")
                    if rep is None or app != "MachineApplicable":
                        continue
                    fn = sp.get("file_name")
                    if not fn or not str(fn).endswith(".rs"):
                        continue
                    path = Path(fn)
                    if not path.is_absolute():
                        path = ROOT / path
                    edits.setdefault(path, []).append(
                        (
                            sp.get("line_start", 0),
                            sp.get("column_start", 0),
                            sp.get("column_end", 0),
                            rep,
                        )
                    )
                walk(child)

        walk(msg)
    return edits


def apply_edits(edits: dict[Path, list[tuple[int, int, int, str]]]) -> int:
    n = 0
    for path, items in edits.items():
        if not path.exists():
            continue
        lines = path.read_text(encoding="utf-8").splitlines()
        # sort by line desc, col desc
        for line_no, c0, c1, rep in sorted(items, key=lambda x: (-x[0], -x[1])):
            if line_no <= 0 or line_no > len(lines):
                continue
            line = lines[line_no - 1]
            if c0 <= 0 or c1 <= 0 or c0 > len(line) + 1:
                continue
            # columns are 1-based
            new_line = line[: c0 - 1] + rep + line[c1 - 1 :]
            if new_line != line:
                lines[line_no - 1] = new_line
                n += 1
        content = "\n".join(lines)
        if path.read_text(encoding="utf-8").endswith("\n"):
            content += "\n"
        path.write_text(content, encoding="utf-8")
    return n


def count_errors() -> int:
    proc = subprocess.run(
        ["cargo", "check", "--message-format=json"],
        cwd=ROOT,
        env=ENV,
        capture_output=True,
        text=True,
        errors="replace",
    )
    c = 0
    for raw in proc.stdout.splitlines():
        try:
            o = json.loads(raw)
        except json.JSONDecodeError:
            continue
        if o.get("reason") == "compiler-message" and o.get("message", {}).get("level") == "error":
            c += 1
    return c


def main() -> None:
    for i in range(15):
        before = count_errors()
        if before == 0:
            print("SUCCESS")
            return
        edits = collect_suggestions()
        n = apply_edits(edits)
        after = count_errors()
        print(f"iter {i+1}: suggestions={n} errors {before}->{after}")
        if n == 0:
            break


if __name__ == "__main__":
    main()
