#!/usr/bin/env python3
"""Validate self-hosted lexer .kbc output (op counts, exports, quick sanity)."""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
KBC = ROOT / "_lexer_full_out.kbc"


def main() -> int:
    if not KBC.is_file():
        print(f"Missing {KBC}")
        return 1
    text = KBC.read_text(encoding="utf-8")
    if not text.startswith("kabootar-bytecode/1"):
        print("Bad header")
        return 1
    exports = re.search(r"^exports=(.+)$", text, re.M)
    fn_names = re.findall(r"^fn (\w+)", text, re.M)
    op_blocks = re.findall(
        r"^fn \w+\([^)]*\) \{\n((?:  \w+[^\n]*\n)*)",
        text,
        re.M,
    )
    print(f"exports: {exports.group(1) if exports else '?'}")
    print(f"functions ({len(fn_names)}): {', '.join(fn_names)}")
    for name, block in zip(fn_names, op_blocks):
        ops = [ln.strip() for ln in block.strip().splitlines() if ln.strip()]
        print(f"  {name}: {len(ops)} ops")
    if "lxScan" in fn_names:
        idx = fn_names.index("lxScan")
        n = len([ln for ln in op_blocks[idx].strip().splitlines() if ln.strip()])
        if n < 50:
            print(f"WARN: lxScan only {n} ops (likely truncated emit)")
            return 2
    if "tokenize" not in fn_names:
        print("FAIL: missing tokenize")
        return 1
    print("Sanity OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
