#!/usr/bin/env python3
"""Fix iteration and nested collects for Rc-backed containers."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SKIP = {"target", "target-p6b9", ".git", "tools"}


def should_process(path: Path) -> bool:
    return path.suffix == ".rs" and not (set(path.parts) & SKIP)


def process(text: str) -> str:
    # Iteration over map/array bindings (works for HashMap/Vec and Rc wrappers).
    for name in ("map", "m", "obj", "o", "ns", "headers", "meta", "bindings"):
        text = re.sub(
            rf"for \(([^,]+), ([^)]+)\) in {name}(?!\.iter\(\)) \{{",
            rf"for (\1, \2) in {name}.iter() {{",
            text,
        )
    for name in ("items", "a", "arr", "vals", "rows", "args", "fields"):
        text = re.sub(
            rf"for ([a-zA-Z_][a-zA-Z0-9_]*) in {name}(?!\.iter\(\)) \{{",
            rf"for \1 in {name}.iter() {{",
            text,
        )
    # Nested Value::Array(collect()) in expressions -> from_array
    text = re.sub(
        r"Value::Array\(\s*([^;{}]*?\.collect\(\)\s*)\)",
        r"Value::from_array(\1)",
        text,
        flags=re.S,
    )
    text = re.sub(
        r"Value::Object\(\s*([^;{}]*?\.collect\(\)\s*)\)",
        r"Value::from_object(\1)",
        text,
        flags=re.S,
    )
    return text


def add_rc_import(path: Path, text: str) -> str:
    if "Rc::make_mut" not in text and "Rc::new" not in text:
        return text
    if "use std::rc::Rc" in text:
        return text
    lines = text.splitlines(keepends=True)
    insert_at = 0
    for i, line in enumerate(lines):
        if line.startswith("use ") or line.startswith("pub use "):
            insert_at = i + 1
    lines.insert(insert_at, "use std::rc::Rc;\n")
    return "".join(lines)


def main() -> None:
    n = 0
    for path in ROOT.rglob("*.rs"):
        if not should_process(path):
            continue
        original = path.read_text(encoding="utf-8")
        text = process(original)
        text = add_rc_import(path, text)
        if text != original:
            path.write_text(text, encoding="utf-8")
            n += 1
    print(f"updated {n} files")


if __name__ == "__main__":
    main()
