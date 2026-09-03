#!/usr/bin/env python3
"""Mechanical codemod: wrap Value::Array/Object args with from_array/from_object helpers."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SKIP = {"target", "target-p6b9", ".git"}


def should_process(path: Path) -> bool:
    if path.suffix != ".rs":
        return False
    parts = set(path.parts)
    if parts & SKIP:
        return False
    return True


def find_matching_paren(s: str, open_idx: int) -> int:
    depth = 0
    i = open_idx
    while i < len(s):
        c = s[i]
        if c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                return i
        elif c in "\"'":
            quote = c
            i += 1
            while i < len(s):
                if s[i] == "\\":
                    i += 2
                    continue
                if s[i] == quote:
                    break
                i += 1
        i += 1
    raise ValueError(f"unmatched paren at {open_idx}")


def replace_ctor(text: str, variant: str, helper: str) -> str:
    needle = f"Value::{variant}("
    out: list[str] = []
    i = 0
    while True:
        j = text.find(needle, i)
        if j == -1:
            out.append(text[i:])
            break
        out.append(text[i:j])
        arg_start = j + len(needle)
        # Already wrapped?
        if text[arg_start : arg_start + 4] == "Rc::":
            out.append(needle)
            i = arg_start
            continue
        if text[arg_start : arg_start + len(helper) + 1] == f"{helper}(":
            out.append(needle)
            i = arg_start
            continue
        close = find_matching_paren(text, arg_start - 1)
        arg = text[arg_start:close]
        out.append(f"Value::{helper}({arg})")
        i = close + 1
    return "".join(out)


def fix_mut_destructure(text: str) -> str:
    # Value::Array(mut name) in match/if let -> ref mut + make_mut at block start
    pattern = re.compile(
        r"(?P<prefix>\b(?:match|if let)\b[^{;]*?Value::Array\()mut (?P<name>\w+)\)"
    )

    def repl(m: re.Match[str]) -> str:
        return f"{m.group('prefix')}ref mut {m.group('name')})"

    return pattern.sub(repl, text)


def process_file(path: Path) -> bool:
    original = path.read_text(encoding="utf-8")
    text = original
    text = replace_ctor(text, "Array", "from_array")
    text = replace_ctor(text, "Object", "from_object")
    if text != original:
        path.write_text(text, encoding="utf-8")
        return True
    return False


def main() -> None:
    changed = 0
    for path in ROOT.rglob("*.rs"):
        if not should_process(path):
            continue
        if process_file(path):
            changed += 1
            print(path.relative_to(ROOT))
    print(f"changed {changed} files")


if __name__ == "__main__":
    main()
