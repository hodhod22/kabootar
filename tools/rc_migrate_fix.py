#!/usr/bin/env python3
"""Robust Rc Value migration fixer — run in loop with cargo check."""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "src"
SKIP_PARTS = {"target", "target-p6b9", ".git", "tools"}


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
            q = c
            i += 1
            while i < len(s):
                if s[i] == "\\":
                    i += 2
                    continue
                if s[i] == q:
                    break
                i += 1
        i += 1
    raise ValueError(f"unmatched paren at {open_idx}")


def line_starts(text: str, pos: int) -> int:
    return text.rfind("\n", 0, pos) + 1


def line_at(text: str, pos: int) -> str:
    start = line_starts(text, pos)
    end = text.find("\n", pos)
    if end == -1:
        end = len(text)
    return text[start:end]


def is_pattern_position(text: str, pos: int) -> bool:
    """True if Value::Array/Object at pos is in a pattern context."""
    line = line_at(text, pos)
    col = pos - line_starts(text, pos)
    prefix = line[:col].rstrip()

    if re.search(r"=>$", prefix):
        return True
    if re.match(r"^\s*\|", line):
        return True
    if re.search(r"\bmatches!\(", line):
        return True
    if re.search(r"\b(let|if let|while let)(\s+mut)?\s+Value::(Array|Object)\(", line):
        return True
    if re.search(r"\b(let|if let|while let)(\s+mut)?\s*\(", line) and "Value::" in line:
        return True
    # Second tuple pattern element only: (Type, Value::Array(...))
    if ", Value::Array(" in line or ", Value::Object(" in line:
        if re.search(r"\b(let|if let|while let)\b", line):
            return True
    return False


def wrap_ctor(text: str, variant: str, helper: str) -> tuple[str, int]:
    needle = f"Value::{variant}("
    out: list[str] = []
    i = 0
    changes = 0
    while True:
        j = text.find(needle, i)
        if j == -1:
            out.append(text[i:])
            break
        arg_start = j + len(needle)
        arg_prefix = text[arg_start : arg_start + 20]
        if arg_prefix.startswith("Rc::") or arg_prefix.startswith("from_"):
            out.append(text[i : arg_start])
            i = arg_start
            continue
        if is_pattern_position(text, j):
            out.append(text[i : arg_start])
            i = arg_start
            continue
        close = find_matching_paren(text, arg_start - 1)
        arg = text[arg_start:close]
        out.append(text[i:j])
        out.append(f"Value::{helper}({arg})")
        changes += 1
        i = close + 1
    return "".join(out), changes


def fix_mut_bindings(text: str) -> tuple[str, int]:
    changes = 0
    new = text
    for variant in ("Array", "Object"):
        pat = rf"\bValue::{variant}\(mut ([a-zA-Z_][a-zA-Z0-9_]*)\)"
        repl = rf"Value::{variant}(ref mut \1)"
        new2, n = re.subn(pat, repl, new)
        new = new2
        changes += n
    return new, changes


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
    "drain",
    "append",
)


def fix_make_mut_calls(text: str) -> tuple[str, int]:
    """Fix lines that call methods on ref mut Rc bindings without make_mut."""
    lines = text.splitlines(keepends=True)
    rc_mut: set[str] = set()
    changes = 0
    out: list[str] = []

    for line in lines:
        for m in re.finditer(
            r"Value::(?:Array|Object)\(ref mut ([a-zA-Z_][a-zA-Z0-9_]*)\)", line
        ):
            rc_mut.add(m.group(1))

        modified = line
        for name in list(rc_mut):
            # skip if already make_mut
            if f"Rc::make_mut({name})" in modified:
                continue
            idx_pat = re.compile(
                rf"^(\s*){re.escape(name)}\[([^\]]+)\]\s*="
            )
            im = idx_pat.match(modified)
            if im:
                modified = (
                    f"{im.group(1)}Rc::make_mut({name})[{im.group(2)}] ="
                    + modified.split("=", 1)[1]
                )
                changes += 1
                continue
            for meth in MUT_METHODS:
                call_pat = re.compile(
                    rf"^(\s*){re.escape(name)}\.{meth}\("
                )
                cm = call_pat.match(modified.lstrip())
                if cm:
                    indent = modified[: len(modified) - len(modified.lstrip())]
                    rest = modified.lstrip()[len(f"{name}.{meth}") :]
                    modified = f"{indent}Rc::make_mut({name}).{meth}{rest}"
                    changes += 1
                    break

        out.append(modified)

    return "".join(out), changes


def fix_for_loops(text: str) -> tuple[str, int]:
    changes = 0
    new = text
    for name in (
        "map",
        "m",
        "obj",
        "o",
        "ns",
        "headers",
        "meta",
        "bindings",
        "table",
        "fields",
    ):
        pat = rf"for \(([^,]+), ([^)]+)\) in {name}(?!\.iter\(\)|\.into_iter\(\)) \{{"
        new2, n = re.subn(pat, rf"for (\1, \2) in {name}.iter() {{", new)
        new = new2
        changes += n
    for name in ("items", "a", "arr", "vals", "rows", "args", "list", "elems"):
        pat = rf"for ([a-zA-Z_][a-zA-Z0-9_]*) in {name}(?!\.iter\(\)|\.into_iter\(\)) \{{"
        new2, n = re.subn(pat, rf"for \1 in {name}.iter() {{", new)
        new = new2
        changes += n
    return new, changes


def fix_nested_collect(text: str) -> tuple[str, int]:
    changes = 0
    new = text
    for variant, helper in (("Array", "from_array"), ("Object", "from_object")):
        pat = rf"Value::{variant}\(\s*([^;{{}}]*?\.collect\(\)\s*)\)"
        new2, n = re.subn(pat, rf"Value::{helper}(\1)", new, flags=re.S)
        new = new2
        changes += n
    return new, changes


def fix_pattern_from_helpers(text: str) -> tuple[str, int]:
    """Revert from_* accidentally used in patterns."""
    changes = 0
    new = text
    subs = [
        (r"\blet mut Value::from_array\(", "let mut Value::Array("),
        (r"\blet mut Value::from_object\(", "let mut Value::Object("),
        (r"\blet Value::from_array\(", "let Value::Array("),
        (r"\blet Value::from_object\(", "let Value::Object("),
        (r"\bif let Value::from_array\(", "if let Value::Array("),
        (r"\bif let Value::from_object\(", "if let Value::Object("),
        (r"\bwhile let Value::from_array\(", "while let Value::Array("),
        (r"\bwhile let Value::from_object\(", "while let Value::Object("),
        (r"\| Value::from_array\(", "| Value::Array("),
        (r"\| Value::from_object\(", "| Value::Object("),
        (r"matches!\(([^)]*),\s*Value::from_array\(", r"matches!(\1, Value::Array("),
        (r"matches!\(([^)]*),\s*Value::from_object\(", r"matches!(\1, Value::Object("),
        (r"\(_, Value::from_array\(", "(_, Value::Array("),
        (r"\(_, Value::from_object\(", "(_, Value::Object("),
        (r"\(SqlType::Json, Value::from_array\(", "(SqlType::Json, Value::Array("),
        (r"\(SqlType::Json, Value::from_object\(", "(SqlType::Json, Value::Object("),
        (r"Some\(crate::value::Value::from_array\(", "Some(crate::value::Value::Array("),
        (r"Some\(crate::value::Value::from_object\(", "Some(crate::value::Value::Object("),
        (r"\blet Some\(Value::from_array\(", "let Some(Value::Array("),
        (r"\blet Some\(Value::from_object\(", "let Some(Value::Object("),
        (r"\bif let Some\(Value::from_array\(", "if let Some(Value::Array("),
        (r"\bif let Some\(Value::from_object\(", "if let Some(Value::Object("),
        (r"\bwhile let Some\(Value::from_array\(", "while let Some(Value::Array("),
        (r"\bwhile let Some\(Value::from_object\(", "while let Some(Value::Object("),
        (r"\| Some\(Value::from_array\(", "| Some(Value::Array("),
        (r"\| Some\(Value::from_object\(", "| Some(Value::Object("),
    ]
    # ColKind / Pattern tuple patterns
    new = re.sub(r"\((ColKind::[^,]+), Value::from_array\(", r"(\1, Value::Array(", new)
    new = re.sub(r"\((ColKind::[^,]+), Value::from_object\(", r"(\1, Value::Object(", new)
    new = re.sub(r"\((Pattern::Array\([^)]*\)), Value::from_array\(", r"(\1, Value::Array(", new)
    new = re.sub(r"\((Pattern::Object\([^)]*\)), Value::from_object\(", r"(\1, Value::Object(", new)
    # Match arms: Value::from_* at line start before =>
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
    for pat, repl in subs:
        new2, n = re.subn(pat, repl, new)
        new = new2
        changes += n
    return new, changes


def add_rc_import(text: str) -> tuple[str, int]:
    if "Rc::" not in text and "Rc<" not in text:
        return text, 0
    if re.search(r"use std::rc::Rc\b", text):
        return text, 0
    lines = text.splitlines(keepends=True)
    insert_at = 0
    for i, line in enumerate(lines):
        if line.startswith("use ") or line.startswith("pub use "):
            insert_at = i + 1
    lines.insert(insert_at, "use std::rc::Rc;\n")
    return "".join(lines), 1


def fix_serde_json_false_positives(text: str) -> tuple[str, int]:
    changes = 0
    subs = [
        ("serde_json::Value::from_array(", "serde_json::Value::Array("),
        ("serde_json::Value::from_object(", "serde_json::Value::Object("),
    ]
    new = text
    for old, repl in subs:
        new2, n = new.replace(old, repl), new.count(old)
        new = new2
        changes += n
    return new, changes


def fix_safe_expressions(text: str) -> tuple[str, int]:
    changes = 0
    subs = [
        (r"push_stack\(stack, Value::Array\(", "push_stack(stack, Value::from_array("),
        (r"push_stack\(stack, Value::Object\(", "push_stack(stack, Value::from_object("),
        (r"\bOk\(Value::Array\(", "Ok(Value::from_array("),
        (r"\bOk\(Value::Object\(", "Ok(Value::from_object("),
        (r"\breturn Value::Array\(", "return Value::from_array("),
        (r"\breturn Value::Object\(", "return Value::from_object("),
        # Do NOT rewrite Some(Value::Array/Object — often match patterns, not expressions.
        (r"(env\.set\([^,]+,\s*)Value::Array\(", r"\1Value::from_array("),
        (r"(env\.set\([^,]+,\s*)Value::Object\(", r"\1Value::from_object("),
        (r"Value::Array\(([^)]*\.collect\(\))\)", r"Value::from_array(\1)"),
        (r"Value::Object\(([^)]*\.collect\(\))\)", r"Value::from_object(\1)"),
        (r"Value::Array\(([^)]*\.collect::<[^>]+>\(\))\)", r"Value::from_array(\1)"),
        (r"Value::Object\(([^)]*\.collect::<[^>]+>\(\))\)", r"Value::from_object(\1)"),
    ]
    new = text
    for pat, repl in subs:
        new2, n = re.subn(pat, repl, new)
        new = new2
        changes += n
    return new, changes


def process_file(path: Path) -> int:
    original = path.read_text(encoding="utf-8")
    text = original
    total = 0

    text, n = fix_pattern_from_helpers(text)
    total += n

    for variant, helper in (("Array", "from_array"), ("Object", "from_object")):
        text, n = wrap_ctor(text, variant, helper)
        total += n

    text, n = fix_nested_collect(text)
    total += n

    text, n = fix_mut_bindings(text)
    total += n

    text, n = fix_make_mut_calls(text)
    total += n

    text, n = fix_for_loops(text)
    total += n

    text, n = fix_safe_expressions(text)
    total += n

    text, n = fix_serde_json_false_positives(text)
    total += n

    text, n = fix_pattern_from_helpers(text)
    total += n

    text, n = add_rc_import(text)
    total += n

    if text != original:
        path.write_text(text, encoding="utf-8")
    return total


def process_all_src() -> int:
    total = 0
    for path in SRC.rglob("*.rs"):
        if set(path.parts) & SKIP_PARTS:
            continue
        total += process_file(path)
    return total


def cargo_errors() -> tuple[int, dict[str, int]]:
    env = {
        **dict(subprocess.os.environ),
        "CARGO_TARGET_DIR": str(ROOT / "target-p6b9"),
        "PATH": "/c/Users/hodho/.cargo/bin:" + subprocess.os.environ.get("PATH", ""),
    }
    proc = subprocess.run(
        ["cargo", "check", "--message-format=json"],
        cwd=ROOT,
        env=env,
        capture_output=True,
        text=True,
        errors="replace",
    )
    counts: dict[str, int] = {}
    total = 0
    for line in proc.stdout.splitlines():
        if not line.strip().startswith("{"):
            continue
        try:
            o = json.loads(line)
        except json.JSONDecodeError:
            continue
        if o.get("reason") != "compiler-message":
            continue
        m = o.get("message", {})
        if m.get("level") != "error":
            continue
        code = m.get("code", {}).get("code", "?")
        counts[code] = counts.get(code, 0) + 1
        total += 1
    return total, counts


def apply_json_hints() -> int:
    """Use cargo JSON errors to apply targeted fixes."""
    env = {
        **dict(subprocess.os.environ),
        "CARGO_TARGET_DIR": str(ROOT / "target-p6b9"),
        "PATH": "/c/Users/hodho/.cargo/bin:" + subprocess.os.environ.get("PATH", ""),
    }
    proc = subprocess.run(
        ["cargo", "check", "--message-format=json"],
        cwd=ROOT,
        env=env,
        capture_output=True,
        text=True,
        errors="replace",
    )
    file_changes: dict[Path, list[tuple[int, str, str]]] = {}
    for line in proc.stdout.splitlines():
        if not line.strip().startswith("{"):
            continue
        try:
            o = json.loads(line)
        except json.JSONDecodeError:
            continue
        if o.get("reason") != "compiler-message":
            continue
        m = o.get("message", {})
        if m.get("level") != "error":
            continue
        code = m.get("code", {}).get("code")
        msg = m.get("message", "")
        sp = (m.get("spans") or [{}])[0]
        f = sp.get("file_name")
        if not f or not f.endswith(".rs"):
            continue
        path = Path(f)
        if not path.is_absolute():
            path = ROOT / path
        line_no = sp.get("line_start", 0)

        if code == "E0308" and "expected struct `Rc" in msg:
            file_changes.setdefault(path, []).append((line_no, "wrap_ctor", msg))
        elif code == "E0308" and "expected struct `Vec" in msg:
            file_changes.setdefault(path, []).append((line_no, "to_vec", msg))
        elif code == "E0596":
            file_changes.setdefault(path, []).append((line_no, "make_mut", msg))

    changes = 0
    for path, hints in file_changes.items():
        if not path.exists():
            continue
        lines = path.read_text(encoding="utf-8").splitlines()
        for line_no, kind, _msg in hints:
            if line_no <= 0 or line_no > len(lines):
                continue
            idx = line_no - 1
            old = lines[idx]
            new = old
            if kind == "wrap_ctor":
                new = re.sub(r"\bValue::Array\(", "Value::from_array(", old)
                new = re.sub(r"\bValue::Object\(", "Value::from_object(", new)
            elif kind == "to_vec" and ".to_vec()" not in old:
                # append .to_vec() to identifier args in common call patterns
                new = re.sub(
                    r"(\bcall_value\([^,]+,\s*)([a-zA-Z_][a-zA-Z0-9_]*)(\s*,)",
                    r"\1\2.to_vec()\3",
                    old,
                )
                new = re.sub(
                    r"(\binstantiate_class\([^,]+,\s*[^,]+,\s*)([a-zA-Z_][a-zA-Z0-9_]*)(\s*,)",
                    r"\1\2.to_vec()\3",
                    new,
                )
            if new != old:
                lines[idx] = new
                changes += 1
        content = "\n".join(lines)
        if path.read_text(encoding="utf-8").endswith("\n"):
            content += "\n"
        path.write_text(content, encoding="utf-8")
    return changes


def main() -> int:
    max_iters = int(sys.argv[1]) if len(sys.argv) > 1 else 8
    for i in range(max_iters):
        n = process_all_src()
        n += apply_json_hints()
        err_total, counts = cargo_errors()
        print(f"iter {i+1}: file_edits={n} errors={err_total} {dict(sorted(counts.items()))}")
        if err_total == 0:
            print("SUCCESS")
            return 0
        if n == 0 and i > 0:
            print("No progress, stopping")
            break
    err_total, _ = cargo_errors()
    return 1 if err_total else 0


if __name__ == "__main__":
    raise SystemExit(main())
