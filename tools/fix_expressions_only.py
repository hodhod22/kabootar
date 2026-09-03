#!/usr/bin/env python3
"""Safe two-pass Rc constructor migration for expressions only."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "src"

REVERT = [
    (r"Value::from_array\(", "Value::Array("),
    (r"Value::from_object\(", "Value::Object("),
    (r"crate::value::Value::from_array\(", "crate::value::Value::Array("),
    (r"crate::value::Value::from_object\(", "crate::value::Value::Object("),
]

EXPR_WRAP = [
    (r"\bOk\(Value::Array\(", "Ok(Value::from_array("),
    (r"\bOk\(Value::Object\(", "Ok(Value::from_object("),
    (r"\bErr\(Value::Array\(", "Err(Value::from_array("),
    (r"\bErr\(Value::Object\(", "Err(Value::from_object("),
    (r"\breturn Value::Array\(", "return Value::from_array("),
    (r"\breturn Value::Object\(", "return Value::from_object("),
    (r",\s*Value::Array\(", ", Value::from_array("),
    (r",\s*Value::Object\(", ", Value::from_object("),
    (r"Box::new\(Value::Array\(", "Box::new(Value::from_array("),
    (r"Box::new\(Value::Object\(", "Box::new(Value::from_object("),
    (r"Value::Array\(([^)]*\.collect\(\))\)", r"Value::from_array(\1)"),
    (r"Value::Object\(([^)]*\.collect\(\))\)", r"Value::from_object(\1)"),
    (r"Value::Array\(([^)]*\.collect::<[^>]+>\(\))\)", r"Value::from_array(\1)"),
    (r"Value::Object\(([^)]*\.collect::<[^>]+>\(\))\)", r"Value::from_object(\1)"),
    (r"Value::Array\(([^)]*\.to_vec\(\))\)", r"Value::from_array(\1)"),
    (r"=\s*Value::Array\(vec!", "= Value::from_array(vec!"),
    (r"=\s*Value::Object\(HashMap::", "= Value::from_object(HashMap::"),
    (r"=\s*Value::Object\(\s*\{", "= Value::from_object({"),
    (r"=\s*Value::Array\(\[", "= Value::from_array(["),
]

PATTERN_FIX = [
    (r"\(Value::from_array\(([^)]*)\),\s*Value::", r"(Value::Array(\1), Value::"),
    (r"\(Value::from_object\(([^)]*)\),\s*Value::", r"(Value::Object(\1), Value::"),
    (r"\(Value::Object\(([^)]*)\), Value::from_object\(", r"(Value::Object(\1), Value::Object("),
    (r"\(Value::Array\(([^)]*)\), Value::from_array\(", r"(Value::Array(\1), Value::Array("),
    (r"\| Value::from_array\(", "| Value::Array("),
    (r"\| Value::from_object\(", "| Value::Object("),
    (r"\blet Value::from_array\(", "let Value::Array("),
    (r"\blet Value::from_object\(", "let Value::Object("),
    (r"\blet mut Value::from_array\(", "let mut Value::Array("),
    (r"\blet mut Value::from_object\(", "let mut Value::Object("),
    (r"\bif let Value::from_array\(", "if let Value::Array("),
    (r"\bif let Value::from_object\(", "if let Value::Object("),
    (r"\bwhile let Value::from_array\(", "while let Value::Array("),
    (r"\bwhile let Value::from_object\(", "while let Value::Object("),
    (r"\blet Some\(Value::from_array\(", "let Some(Value::Array("),
    (r"\blet Some\(Value::from_object\(", "let Some(Value::Object("),
    (r"\bif let Some\(Value::from_array\(", "if let Some(Value::Array("),
    (r"\bif let Some\(Value::from_object\(", "if let Some(Value::Object("),
    (r"Some\(Value::from_array\(([^)]*)\)\s*=>", r"Some(Value::Array(\1)) =>"),
    (r"Some\(Value::from_object\(([^)]*)\)\s*=>", r"Some(Value::Object(\1)) =>"),
    (r"Value::from_array\(([^)]*)\)\s+if\b", r"Value::Array(\1) if"),
    (r"Value::from_object\(([^)]*)\)\s+if\b", r"Value::Object(\1) if"),
    (r"(?m)^(\s+)Value::from_array\(([^)]*)\)\s*=>", r"\1Value::Array(\2) =>"),
    (r"(?m)^(\s+)Value::from_object\(([^)]*)\)\s*=>", r"\1Value::Object(\2) =>"),
    (r"matches!\(([^)]*),\s*Value::from_array\(", r"matches!(\1, Value::Array("),
    (r"matches!\(([^)]*),\s*Value::from_object\(", r"matches!(\1, Value::Object("),
    (r"\(_, Value::from_array\(", "(_, Value::Array("),
    (r"\(_, Value::from_object\(", "(_, Value::Object("),
    (r"\(SqlType::Json, Value::from_array\(", "(SqlType::Json, Value::Array("),
    (r"\(SqlType::Json, Value::from_object\(", "(SqlType::Json, Value::Object("),
    (r"\(ColKind::[^,]+,\s*Value::from_array\(", lambda m: m.group(0).replace("from_array", "Array", 1)),
    (r"\(ColKind::[^,]+,\s*Value::from_object\(", lambda m: m.group(0).replace("from_object", "Object", 1)),
]

SERDE = [
    ("serde_json::Value::from_array(", "serde_json::Value::Array("),
    ("serde_json::Value::from_object(", "serde_json::Value::Object("),
]


def process(text: str) -> str:
    new = text
    for pat, repl in REVERT:
        new = re.sub(pat, repl, new)
    for pat, repl in EXPR_WRAP:
        new = re.sub(pat, repl, new)
    for pat, repl in PATTERN_FIX:
        if callable(repl):
            new = re.sub(pat, repl, new)
        else:
            new = re.sub(pat, repl, new)
    for old, repl in SERDE:
        new = new.replace(old, repl)
    return new


def main() -> None:
    files = 0
    for path in ROOT.rglob("*.rs"):
        old = path.read_text(encoding="utf-8")
        new = process(old)
        if new != old:
            path.write_text(new, encoding="utf-8")
            files += 1
    print(f"updated {files} files")


if __name__ == "__main__":
    main()
