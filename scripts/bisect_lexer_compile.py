#!/usr/bin/env python3
"""Find lexer.kab line prefix that fails self-hosted compile() (os_mount path)."""
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
KAB = os.path.join(ROOT, "target-alt2", "debug", "kabootar.exe")
if not os.path.isfile(KAB):
    KAB = os.path.join(ROOT, "target", "debug", "kabootar.exe")
LEXER = os.path.join(ROOT, "self_host", "lexer.kab")
SRC = os.path.join(ROOT, "_bisect_lexer.kab")
PROBE = os.path.join(ROOT, "self_host", "_bisect_probe.kab")
MANIFEST = ROOT.replace("\\", "/")


def try_compile(n_lines: int, lines: list[str]) -> tuple[bool, str]:
    with open(SRC, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(lines[:n_lines]))
        if not lines[:n_lines][-1].endswith("\n"):
            f.write("\n")
    probe = (
        'import "self_host/compile"\n'
        f'os_mount("/proj", "{MANIFEST}")\n'
        'let kbc = compile(read_text_file("/proj/_bisect_lexer.kab"))\n'
        "return len(kbc)\n"
    )
    with open(PROBE, "w", encoding="utf-8", newline="\n") as f:
        f.write(probe)
    r = subprocess.run(
        [KAB, PROBE],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=900,
    )
    err = (r.stderr or r.stdout or "").strip()
    ok = r.returncode == 0 and "Error:" not in err and "kab_throw" not in err
    return ok, err[:400]


def main() -> int:
    lines = open(LEXER, encoding="utf-8").read().splitlines()
    n = len(lines)
    ok, err = try_compile(n, lines)
    print(f"full ({n} lines): {'OK' if ok else 'FAIL'}")
    if not ok:
        print(err)
    lo, hi = 1, n
    fail_at = n
    while lo <= hi:
        mid = (lo + hi) // 2
        ok, err = try_compile(mid, lines)
        print(f"lines 1..{mid}: {'OK' if ok else 'FAIL'}")
        if ok:
            lo = mid + 1
        else:
            fail_at = mid
            hi = mid - 1
    print(f"\nfirst failing prefix ends near line {fail_at}")
    for i in range(max(0, fail_at - 2), min(n, fail_at + 2)):
        print(f"  {i+1}: {lines[i][:120]}")
    for p in (SRC, PROBE):
        try:
            os.remove(p)
        except OSError:
            pass
    return 0


if __name__ == "__main__":
    sys.exit(main())
