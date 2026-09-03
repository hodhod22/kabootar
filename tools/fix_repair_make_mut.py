#!/usr/bin/env python3
"""Repair Rc::make_mut(var)(args) -> Rc::make_mut(var).method(args) via git diff."""

from __future__ import annotations

import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BROKEN = re.compile(r"Rc::make_mut\((?P<var>[a-zA-Z_][a-zA-Z0-9_]*)\)\(")


def git_diff(path: Path) -> str:
    rel = path.relative_to(ROOT).as_posix()
    return subprocess.check_output(
        ["git", "diff", "-U0", "--", rel], cwd=ROOT, text=True, errors="replace"
    )


def repair_file(path: Path) -> bool:
    text = path.read_text(encoding="utf-8")
    if "Rc::make_mut(" not in text or BROKEN.search(text) is None:
        return False
    diff = git_diff(path)
    replacements: dict[str, str] = {}
    removed = None
    for line in diff.splitlines():
        if line.startswith("-") and not line.startswith("---"):
            removed = line[1:].strip()
        elif line.startswith("+") and not line.startswith("+++"):
            added = line[1:].strip()
            if removed and "Rc::make_mut(" in added and BROKEN.search(added):
                m_old = re.search(
                    r"(?P<var>[a-zA-Z_][a-zA-Z0-9_]*)\.(?P<meth>[a-zA-Z_][a-zA-Z0-9_]*)\(",
                    removed,
                )
                m_new = BROKEN.search(added)
                if m_old and m_new and m_old.group("var") == m_new.group("var"):
                    fixed = added.replace(
                        f"Rc::make_mut({m_old.group('var')})(",
                        f"Rc::make_mut({m_old.group('var')}).{m_old.group('meth')}(",
                        1,
                    )
                    replacements[added] = fixed
            removed = None
    if not replacements:
        return False
    new_text = text
    for bad, good in replacements.items():
        new_text = new_text.replace(bad, good)
    if new_text != text:
        path.write_text(new_text, encoding="utf-8")
        return True
    return False


def main() -> None:
    n = 0
    for path in ROOT.rglob("*.rs"):
        if "target" in path.parts:
            continue
        if repair_file(path):
            n += 1
            print(path.relative_to(ROOT))
    print(f"repaired {n} files")


if __name__ == "__main__":
    main()
