#!/usr/bin/env python3
"""Bisect lexer.kab compile failure via os_mount (same path as CI)."""
import argparse
import os
import subprocess
import sys
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
KAB = os.path.join(ROOT, "target-alt2", "debug", "kabootar.exe")
if not os.path.isfile(KAB):
    KAB = os.path.join(ROOT, "target", "debug", "kabootar.exe")
LEXER = os.path.join(ROOT, "self_host", "lexer.kab")
SRC = os.path.join(ROOT, "_bisect_lexer.kab")
PROBE = os.path.join(ROOT, "self_host", "_bisect_probe.kab")
MANIFEST = ROOT.replace("\\", "/")

STUBS = (
    "\n\npub fn tokenize(source) { return [] }\n"
    "pub fn token_type_name(tok) { return tok.type }\n"
    "pub fn token_value(tok) { return tok.value }\n"
)


def wrap_prefix(lines: list[str]) -> str:
    text = "\n".join(lines)
    need = text.count("{") - text.count("}")
    if need > 0:
        text += "\n" + ("}" * need)
    if "pub fn tokenize" not in text:
        text += STUBS
    return text + "\n"


def timeout_for(n_lines: int) -> int:
    return max(120, int(n_lines * 6))


def try_compile(n_lines: int, lines: list[str]) -> tuple[bool, str, float]:
    src = wrap_prefix(lines[:n_lines])
    with open(SRC, "w", encoding="utf-8", newline="\n") as f:
        f.write(src)
    probe = (
        'import "self_host/compile"\n'
        f'os_mount("/proj", "{MANIFEST}")\n'
        'let kbc = compile(read_text_file("/proj/_bisect_lexer.kab"))\n'
        "return len(kbc)\n"
    )
    with open(PROBE, "w", encoding="utf-8", newline="\n") as f:
        f.write(probe)
    t0 = time.time()
    try:
        r = subprocess.run(
            [KAB, PROBE],
            cwd=ROOT,
            capture_output=True,
            text=True,
            timeout=timeout_for(n_lines),
        )
    except subprocess.TimeoutExpired:
        return False, f"TIMEOUT after {timeout_for(n_lines)}s", time.time() - t0
    err = (r.stderr or r.stdout or "").strip()
    ok = r.returncode == 0 and "Error:" not in err and "kab_throw" not in err
    return ok, err[:500], time.time() - t0


def run_probe(n_lines: int, lines: list[str]) -> bool:
    print(f"\n--- lines 1..{n_lines} (timeout {timeout_for(n_lines)}s) ---")
    ok, err, elapsed = try_compile(n_lines, lines)
    print(f"  {'OK' if ok else 'FAIL'} in {elapsed:.0f}s")
    if not ok:
        print(f"  {err}")
    return ok


def binary_search(lo: int, hi: int, lines: list[str]) -> int:
    fail_at = hi
    print(f"\n--- binary search {lo}..{hi} ---")
    while lo <= hi:
        mid = (lo + hi) // 2
        ok, _, elapsed = try_compile(mid, lines)
        print(f"  lines 1..{mid}: {'OK' if ok else 'FAIL'} ({elapsed:.0f}s)")
        if ok:
            lo = mid + 1
        else:
            fail_at = mid
            hi = mid - 1
    return fail_at


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--lo", type=int, default=14, help="last known OK prefix length")
    ap.add_argument("--hi", type=int, default=0, help="search upper bound (0 = file end)")
    args = ap.parse_args()

    lines = open(LEXER, encoding="utf-8").read().splitlines()
    n = len(lines)
    hi = args.hi if args.hi > 0 else n
    last_ok = args.lo
    first_fail = hi

    # lxScan ends ~529; tokenize ~531-629
    boundaries = [b for b in (200, 350, 450, 529, 580, 620, hi) if last_ok < b <= hi]
    for b in boundaries:
        ok = run_probe(b, lines)
        if ok:
            last_ok = b
        else:
            first_fail = b
            break

    if last_ok < first_fail - 1:
        first_fail = binary_search(last_ok + 1, first_fail, lines)

    print(f"\n=== first failing prefix ends near line {first_fail} ===")
    for i in range(max(0, first_fail - 3), min(n, first_fail + 2)):
        print(f"  {i+1}: {lines[i][:120]}")

    for p in (SRC, PROBE):
        try:
            os.remove(p)
        except OSError:
            pass
    return 0


if __name__ == "__main__":
    sys.exit(main())
