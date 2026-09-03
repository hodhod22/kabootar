#!/usr/bin/env python3
"""Normalize Value constructors: from_* in expressions only, Array/Object in patterns."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SKIP = {"target", "target-p6b9", ".git", "tools"}


def should_process(path: Path) -> bool:
    return path.suffix == ".rs" and not (set(path.parts) & SKIP)


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


def wrap_expressions(text: str) -> str:
    for variant, helper in (("Array", "from_array"), ("Object", "from_object")):
        needle = f"Value::{variant}("
        out: list[str] = []
        i = 0
        while True:
            j = text.find(needle, i)
            if j == -1:
                out.append(text[i:])
                break
            arg_start = j + len(needle)
            if text[arg_start : arg_start + 4] == "Rc::":
                out.append(text[i : arg_start])
                i = arg_start
                continue
            close = find_matching_paren(text, arg_start - 1)
            out.append(text[i:j])
            arg = text[arg_start:close]
            out.append(f"Value::{helper}({arg})")
            i = close + 1
        text = "".join(out)
    return text


PATTERN_FIXES: list[tuple[str, str]] = [
    (r"\(Value::from_array\(", "(Value::Array("),
    (r"\(Value::from_object\(", "(Value::Object("),
    (r"\blet mut Value::from_array\(", "let mut Value::Array("),
    (r"\blet mut Value::from_object\(", "let mut Value::Object("),
    (r"\blet Value::from_array\(", "let Value::Array("),
    (r"\blet Value::from_object\(", "let Value::Object("),
    (r"\bif let Value::from_array\(", "if let Value::Array("),
    (r"\bif let Value::from_object\(", "if let Value::Object("),
    (r"\bwhile let Value::from_array\(", "while let Value::Array("),
    (r"\bwhile let Value::from_object\(", "while let Value::Object("),
    (r"matches!\(([^)]*),\s*Value::from_array\(", r"matches!(\1, Value::Array("),
    (r"matches!\(([^)]*),\s*Value::from_object\(", r"matches!(\1, Value::Object("),
    (r"\|\s*Value::from_array\(", "| Value::Array("),
    (r"\|\s*Value::from_object\(", "| Value::Object("),
]


def fix_patterns(text: str) -> str:
    # Match-arm patterns at line start (after whitespace)
    text = re.sub(r"(?m)^(\s+)Value::from_array\(", r"\1Value::Array(", text)
    text = re.sub(r"(?m)^(\s+)Value::from_object\(", r"\1Value::Object(", text)
    for pat, repl in PATTERN_FIXES:
        text = re.sub(pat, repl, text)
    return text


def process_file(path: Path) -> bool:
    original = path.read_text(encoding="utf-8")
    text = original.replace("Value::from_array(", "Value::Array(")
    text = text.replace("Value::from_object(", "Value::Object(")
    text = wrap_expressions(text)
    text = fix_patterns(text)
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
