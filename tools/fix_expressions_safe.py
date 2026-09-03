#!/usr/bin/env python3
"""Safe expression-only wraps for Value constructors."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SKIP = {"target", "target-p6b9", ".git", "tools"}

SAFE_PREFIXES = (
    "Ok(",
    "return ",
    "return Ok(",
    "Some(",
    "Err(",
    "push_stack(stack, ",
    "push(",
    "env.set(",
    "map.insert(",
    "m.insert(",
    "o.insert(",
    "ns.insert(",
    "out.insert(",
    "obj.insert(",
    "meta.insert(",
    "bindings.insert(",
    "local_vals[i] = ",
    "roots.push(",
    "roots.extend(",
    "items.push(",
    "vec![",
    "=",
)


def should_process(path: Path) -> bool:
    return path.suffix == ".rs" and not (set(path.parts) & SKIP)


def process(text: str) -> str:
    text = re.sub(r"\bOk\(Value::Array\(", "Ok(Value::from_array(", text)
    text = re.sub(r"\bOk\(Value::Object\(", "Ok(Value::from_object(", text)
    text = re.sub(r"\breturn Value::Array\(", "return Value::from_array(", text)
    text = re.sub(r"\breturn Value::Object\(", "return Value::from_object(", text)
    text = re.sub(r"\bSome\(Value::Array\(", "Some(Value::from_array(", text)
    text = re.sub(r"\bSome\(Value::Object\(", "Some(Value::from_object(", text)
    text = re.sub(r"push_stack\(stack, Value::Array\(", "push_stack(stack, Value::from_array(", text)
    text = re.sub(r"push_stack\(stack, Value::Object\(", "push_stack(stack, Value::from_object(", text)
    # env.set(..., Value::Object(
    text = re.sub(
        r"(env\.set\([^,]+,\s*)Value::Object\(",
        r"\1Value::from_object(",
        text,
    )
    text = re.sub(
        r"(env\.set\([^,]+,\s*)Value::Array\(",
        r"\1Value::from_array(",
        text,
    )
    text = re.sub(
        r"Value::Object\(([^()]+)\)(\s*\n\s*\})",
        r"Value::from_object(\1)\2",
        text,
    )
    text = re.sub(
        r"Value::Array\(([^()]+)\)(\s*\n\s*\})",
        r"Value::from_array(\1)\2",
        text,
    )
    return text


def main() -> None:
    n = 0
    for path in ROOT.rglob("*.rs"):
        if not should_process(path):
            continue
        original = path.read_text(encoding="utf-8")
        updated = process(original)
        if updated != original:
            path.write_text(updated, encoding="utf-8")
            n += 1
    print(f"updated {n} files")


if __name__ == "__main__":
    main()
