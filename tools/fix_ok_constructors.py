#!/usr/bin/env python3
"""Safe bulk constructor and pattern-guard fixes."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "src"


def process(text: str) -> str:
    # Match arm guards / patterns wrongly converted to from_*
    text = re.sub(
        r"Value::from_array\(([^)]*)\)\s+if\b", r"Value::Array(\1) if", text
    )
    text = re.sub(
        r"Value::from_object\(([^)]*)\)\s+if\b", r"Value::Object(\1) if", text
    )
    text = re.sub(
        r"Value::from_object\(_\)\s+if\b", r"Value::Object(_) if", text
    )
    text = re.sub(
        r"\(Value::Object\(([^)]*)\), Value::from_object\(",
        r"(Value::Object(\1), Value::Object(",
        text,
    )
    text = re.sub(
        r"\(Value::Array\(([^)]*)\), Value::from_array\(",
        r"(Value::Array(\1), Value::Array(",
        text,
    )
    text = re.sub(
        r"matches!\(([^)]*),\s*crate::value::Value::from_array\(",
        r"matches!(\1, crate::value::Value::Array(",
        text,
    )
    # Expression constructors (never valid in patterns)
    reps = [
        ("Ok(Value::Object(", "Ok(Value::from_object("),
        ("Ok(Value::Array(", "Ok(Value::from_array("),
        ("return Ok(Value::Object(", "return Ok(Value::from_object("),
        ("return Ok(Value::Array(", "return Ok(Value::from_array("),
        ("return Value::Object(", "return Value::from_object("),
        ("return Value::Array(", "return Value::from_array("),
        ("Some(Value::Object(", "Some(Value::Object("),  # never in expressions - skip
        ("push_stack(stack, Value::Object(", "push_stack(stack, Value::from_object("),
        ("push_stack(stack, Value::Array(", "push_stack(stack, Value::from_array("),
        ("Err(Value::Object(", "Err(Value::from_object("),
        ("Err(Value::Array(", "Err(Value::from_array("),
        ("assert!(matches!(val, crate::value::Value::from_array(", "assert!(matches!(val, crate::value::Value::Array("),
    ]
    for old, new in reps:
        text = text.replace(old, new)
    # serde_json false positives
    text = text.replace("serde_json::Value::from_array(", "serde_json::Value::Array(")
    text = text.replace("serde_json::Value::from_object(", "serde_json::Value::Object(")
    return text


def main() -> None:
    n = 0
    for path in ROOT.rglob("*.rs"):
        if "target" in path.parts:
            continue
        old = path.read_text(encoding="utf-8")
        new = process(old)
        if new != old:
            path.write_text(new, encoding="utf-8")
            n += 1
    print(f"updated {n} files")


if __name__ == "__main__":
    main()
