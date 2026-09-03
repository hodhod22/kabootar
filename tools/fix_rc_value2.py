#!/usr/bin/env python3
"""Revert Value::from_array/from_object in patterns; wrap only expression constructors."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SKIP = {"target", "target-p6b9", ".git", "tools"}

PATTERN_RES = [
    re.compile(r"\(Value::from_array\("),
    re.compile(r"\(Value::from_object\("),
    re.compile(r"\blet Value::from_array\("),
    re.compile(r"\blet Value::from_object\("),
    re.compile(r"\blet mut Value::from_array\("),
    re.compile(r"\blet mut Value::from_object\("),
    re.compile(r"\bif let Value::from_array\("),
    re.compile(r"\bif let Value::from_object\("),
    re.compile(r"\bwhile let Value::from_array\("),
    re.compile(r"\bwhile let Value::from_object\("),
    re.compile(r"^\s+Value::from_array\(", re.M),
    re.compile(r"^\s+Value::from_object\(", re.M),
    re.compile(r"\|\s*Value::from_array\("),
    re.compile(r"\|\s*Value::from_object\("),
]


def should_process(path: Path) -> bool:
    if path.suffix != ".rs":
        return False
    return not (set(path.parts) & SKIP)


def revert_patterns(text: str) -> str:
    text = text.replace("Value::from_array(", "Value::Array(")
    text = text.replace("Value::from_object(", "Value::Object(")
    return text


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


def is_pattern_context(text: str, pos: int) -> bool:
    before = text[max(0, pos - 80) : pos]
    if re.search(r"\b(match|if let|while let)\b[^{};]*$", before):
        return True
    if re.search(r"\blet(\s+mut)?\s*$", before):
        return True
    if re.search(r"=>\s*$", before):
        return True
    if re.search(r"\(\s*$", before) and re.search(
        r"\b(match|if let|while let)\b", text[max(0, pos - 200) : pos]
    ):
        return True
    line_start = text.rfind("\n", 0, pos) + 1
    line = text[line_start:pos]
    if re.match(r"\s+Value::(Array|Object)\(", line):
        # match arm pattern at line start
        chunk = text[max(0, pos - 400) : pos]
        if "match " in chunk or "if let " in chunk or "while let " in chunk:
            return True
    return False


def wrap_constructors(text: str) -> str:
    for variant, helper in (("Array", "from_array"), ("Object", "from_object")):
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
            if text[arg_start : arg_start + 4] == "Rc::":
                out.append(needle)
                i = arg_start
                continue
            if is_pattern_context(text, j):
                out.append(needle)
                i = arg_start
                continue
            close = find_matching_paren(text, arg_start - 1)
            arg = text[arg_start:close]
            out.append(f"Value::{helper}({arg})")
            i = close + 1
        text = "".join(out)
    return text


def process_file(path: Path) -> bool:
    original = path.read_text(encoding="utf-8")
    text = revert_patterns(original)
    text = wrap_constructors(text)
    if text != original:
        path.write_text(text, encoding="utf-8")
        return True
    return False


def main() -> None:
    n = 0
    for path in ROOT.rglob("*.rs"):
        if not should_process(path):
            continue
        if process_file(path):
            n += 1
    print(f"updated {n} files")


if __name__ == "__main__":
    main()
