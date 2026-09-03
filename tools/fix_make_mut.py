#!/usr/bin/env python3
"""Fix Rc container patterns: patterns, make_mut, and from_* on existing Rc."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SKIP = {"target", "target-p6b9", ".git", "tools"}

MUT_METHODS = (
    "push",
    "pop",
    "extend",
    "insert",
    "remove",
    "clear",
    "retain",
    "sort",
    "sort_by",
    "reverse",
    "truncate",
    "resize",
)


def should_process(path: Path) -> bool:
    return path.suffix == ".rs" and not (set(path.parts) & SKIP)


def fix_pattern_second_tuple(text: str) -> str:
    text = re.sub(r", Value::from_array\(", ", Value::Array(", text)
    text = re.sub(r", Value::from_object\(", ", Value::Object(", text)
    text = re.sub(
        r"Some\(crate::value::Value::from_array\(", "Some(crate::value::Value::Array(", text
    )
    text = re.sub(r"\| ColKind::[^,]+, Value::from_array\(", lambda m: m.group(0).replace("from_array", "Array"), text)
    # (_, Value::from_
    text = re.sub(r"\(_, Value::from_array\(", "(_, Value::Array(", text)
    text = re.sub(r"\(_, Value::from_object\(", "(_, Value::Object(", text)
    # (SqlType::Json, Value::from_
    text = re.sub(r"\(SqlType::Json, Value::from_array\(", "(SqlType::Json, Value::Array(", text)
    text = re.sub(r"\(SqlType::Json, Value::from_object\(", "(SqlType::Json, Value::Object(", text)
    # (ColKind..., Value::from_
    text = re.sub(
        r"\((ColKind::[^)]+), Value::from_array\(",
        r"(\1, Value::Array(",
        text,
    )
    text = re.sub(
        r"\((ColKind::[^)]+), Value::from_object\(",
        r"(\1, Value::Object(",
        text,
    )
    return text


def fix_mut_bindings(text: str) -> str:
    text = re.sub(
        r"\bValue::Array\(mut ([a-zA-Z_][a-zA-Z0-9_]*)\)",
        r"Value::Array(ref mut \1)",
        text,
    )
    text = re.sub(
        r"\bValue::Object\(mut ([a-zA-Z_][a-zA-Z0-9_]*)\)",
        r"Value::Object(ref mut \1)",
        text,
    )
    return text


def wrap_mut_calls(text: str) -> str:
    lines = text.splitlines(keepends=True)
    out: list[str] = []
    rc_mut_names: set[str] = set()

    for line in lines:
        m = re.search(
            r"Value::(?:Array|Object)\(ref mut ([a-zA-Z_][a-zA-Z0-9_]*)\)",
            line,
        )
        if m and ("let " in line or "if let " in line or "= (" in line or "=>" in line):
            rc_mut_names.add(m.group(1))

        stripped = line.lstrip()
        for name in list(rc_mut_names):
            # index assign: items[i] = val
            idx_pat = re.compile(
                rf"^(?P<indent>\s*){re.escape(name)}\[(?P<idx>[^\]]+)\]\s*=",
            )
            im = idx_pat.match(line)
            if im:
                line = (
                    f"{im.group('indent')}Rc::make_mut({name})[{im.group('idx')}] ="
                    + line.split("=", 1)[1]
                )
                break

            for meth in MUT_METHODS:
                call_pat = re.compile(
                    rf"^(?P<indent>\s*){re.escape(name)}\.{meth}\(",
                )
                cm = call_pat.match(line)
                if cm:
                    rest = line[cm.end() - 1 :]  # includes (
                    line = f"{cm.group('indent')}Rc::make_mut({name}){rest}"
                    break
            else:
                continue
            break

        out.append(line)

    return "".join(out)


def process_file(path: Path) -> bool:
    original = path.read_text(encoding="utf-8")
    text = original
    text = fix_pattern_second_tuple(text)
    text = fix_mut_bindings(text)
    text = wrap_mut_calls(text)
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
