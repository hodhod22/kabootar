#!/usr/bin/env python3
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "src"
for path in ROOT.rglob("*.rs"):
    text = path.read_text(encoding="utf-8")
    if "Rc::" not in text and "Rc<" not in text:
        continue
    if re.search(r"use std::rc::Rc\b", text):
        continue
    lines = text.splitlines(keepends=True)
    insert_at = 0
    for i, line in enumerate(lines):
        if line.startswith("use ") or line.startswith("pub use "):
            insert_at = i + 1
    lines.insert(insert_at, "use std::rc::Rc;\n")
    path.write_text("".join(lines), encoding="utf-8")
    print(path)
