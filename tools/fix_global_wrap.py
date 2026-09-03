#!/usr/bin/env python3
"""Global wrap then pattern revert — fast path for remaining constructors."""

from __future__ import annotations

import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "src"

# Import pattern revert from sibling
import sys
sys.path.insert(0, str(ROOT / "tools"))
from fix_pattern_revert import process as revert_patterns  # noqa: E402


def wrap_all(text: str) -> str:
    text = text.replace("Value::Array(", "Value::from_array(")
    text = text.replace("Value::Object(", "Value::from_object(")
    text = text.replace("crate::value::Value::Array(", "crate::value::Value::from_array(")
    text = text.replace("crate::value::Value::Object(", "crate::value::Value::from_object(")
    return text


def fix_serde(text: str) -> str:
    text = text.replace("serde_json::Value::from_array(", "serde_json::Value::Array(")
    text = text.replace("serde_json::Value::from_object(", "serde_json::Value::Object(")
    return text


def main() -> None:
    n = 0
    for path in SRC.rglob("*.rs"):
        if path.name == "value.rs":
            continue
        old = path.read_text(encoding="utf-8")
        new = wrap_all(old)
        new = revert_patterns(new)
        new = fix_serde(new)
        if new != old:
            path.write_text(new, encoding="utf-8")
            n += 1
    print(f"updated {n} files")


if __name__ == "__main__":
    main()
