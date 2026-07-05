#!/usr/bin/env python3
"""Quick compile probe for lexer.kab prefix lengths."""
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
PROBE = os.path.join(ROOT, "self_host", "_quick_probe.kab")
MANIFEST = ROOT.replace("\\", "/")

STUBS = (
    "\n\npub fn tokenize(source) { return [] }\n"
    "pub fn token_type_name(tok) { return tok.type }\n"
    "pub fn token_value(tok) { return tok.value }\n"
)


def wrap_prefix(n: int, lines: list[str]) -> str:
    text = "\n".join(lines[:n])
    need = text.count("{") - text.count("}")
    if need > 0:
        text += "\n" + ("}" * need)
    if "pub fn tokenize" not in text:
        text += STUBS
    return text + "\n"


def try_compile(n: int, lines: list[str], timeout: int) -> tuple[bool, str, float]:
    src = wrap_prefix(n, lines)
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
            timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        return False, f"TIMEOUT after {timeout}s", time.time() - t0
    err = (r.stderr or r.stdout or "").strip()
    ok = r.returncode == 0 and "Error:" not in err and "kab_throw" not in err
    return ok, err[:500], time.time() - t0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("lines", nargs="*", type=int, default=[129, 130, 131, 144, 200])
    args = ap.parse_args()
    lines = open(LEXER, encoding="utf-8").read().splitlines()
    for n in args.lines:
        timeout = max(120, n * 8)
        ok, err, elapsed = try_compile(n, lines, timeout)
        print(f"lines 1..{n}: {'OK' if ok else 'FAIL'} in {elapsed:.0f}s")
        if not ok:
            print(f"  {err}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
