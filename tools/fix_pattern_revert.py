#!/usr/bin/env python3
"""Revert from_* in pattern contexts."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "src"


def process(text: str) -> str:
    subs = [
        (r"\blet Some\(Value::from_object\(", "let Some(Value::Object("),
        (r"\blet Some\(Value::from_array\(", "let Some(Value::Array("),
        (r"\bif let Some\(Value::from_object\(", "if let Some(Value::Object("),
        (r"\bif let Some\(Value::from_array\(", "if let Some(Value::Array("),
        (r"\bwhile let Some\(Value::from_object\(", "while let Some(Value::Object("),
        (r"\bwhile let Some\(Value::from_array\(", "while let Some(Value::Array("),
        (r"\blet Value::from_object\(", "let Value::Object("),
        (r"\blet Value::from_array\(", "let Value::Array("),
        (r"\blet mut Value::from_object\(", "let mut Value::Object("),
        (r"\blet mut Value::from_array\(", "let mut Value::Array("),
        (r"\bif let Value::from_object\(", "if let Value::Object("),
        (r"\bif let Value::from_array\(", "if let Value::Array("),
        (r"\| Value::from_object\(", "| Value::Object("),
        (r"\| Value::from_array\(", "| Value::Array("),
        (r"matches!\(([^)]*),\s*Value::from_object\(", r"matches!(\1, Value::Object("),
        (r"matches!\(([^)]*),\s*Value::from_array\(", r"matches!(\1, Value::Array("),
        (r"\(_, Value::from_object\(", "(_, Value::Object("),
        (r"\(_, Value::from_array\(", "(_, Value::Array("),
        (r"\(SqlType::Json, Value::from_object\(", "(SqlType::Json, Value::Object("),
        (r"\(SqlType::Json, Value::from_array\(", "(SqlType::Json, Value::Array("),
    ]
    new = text
    for pat, repl in subs:
        new = re.sub(pat, repl, new)
    new = re.sub(
        r"(?m)^(\s+)Value::from_array\(([^)]*)\)\s*=>",
        r"\1Value::Array(\2) =>",
        new,
    )
    new = re.sub(
        r"(?m)^(\s+)Value::from_object\(([^)]*)\)\s*=>",
        r"\1Value::Object(\2) =>",
        new,
    )
    new = re.sub(
        r"Some\(Value::from_array\(([^)]*)\)\s*=>",
        r"Some(Value::Array(\1)) =>",
        new,
    )
    new = re.sub(
        r"Some\(Value::from_object\(([^)]*)\)\s*=>",
        r"Some(Value::Object(\1)) =>",
        new,
    )
    new = re.sub(
        r"\(Value::Object\(([^)]*)\), Value::from_object\(",
        r"(Value::Object(\1), Value::Object(",
        new,
    )
    new = re.sub(
        r"\(Value::Array\(([^)]*)\), Value::from_array\(",
        r"(Value::Array(\1), Value::Array(",
        new,
    )
    new = re.sub(
        r"\(Value::from_array\(([^)]*)\),\s*Value::",
        r"(Value::Array(\1), Value::",
        new,
    )
    new = re.sub(
        r"\(Value::from_object\(([^)]*)\),\s*Value::",
        r"(Value::Object(\1), Value::",
        new,
    )
    new = re.sub(
        r"Value::from_array\(([^)]*)\)\s+if\b",
        r"Value::Array(\1) if",
        new,
    )
    new = re.sub(
        r"Value::from_object\(([^)]*)\)\s+if\b",
        r"Value::Object(\1) if",
        new,
    )
    return new


def main() -> None:
    n = 0
    for path in ROOT.rglob("*.rs"):
        old = path.read_text(encoding="utf-8")
        new = process(old)
        if new != old:
            path.write_text(new, encoding="utf-8")
            n += 1
    print(f"updated {n} files")


if __name__ == "__main__":
    main()
