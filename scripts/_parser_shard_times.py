#!/usr/bin/env python3
"""Measure self-host compile wall time for parser densify shards."""
from __future__ import annotations

import os
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
KAB = Path(os.environ.get("KABOOTAR_BIN", ROOT / "target-p6b9-rel/release/kabootar.exe"))
MANIFEST = str(ROOT).replace("\\", "/")
if len(MANIFEST) >= 2 and MANIFEST[1] == ":":
    MANIFEST = MANIFEST[0].lower() + MANIFEST[1:]

SHARDS = [
    "parser_util.kab",
    "parser_type_args.kab",
    "parser_unary.kab",
    "parser_mul.kab",
    "parser_add_shift.kab",
    "parser_rel_expr.kab",
    "parser_compare.kab",
    "parser_postfix.kab",
    "parser_stmt.kab",
    "parser_main.kab",
    "parser_impl.kab",
]


def main() -> None:
    env = dict(os.environ)
    env["KABOOTAR_VM"] = "host"
    env["KABOOTAR_COMPILE"] = "rust"
    probe_path = ROOT / "self_host/_profile_probe_gen.kab"
    print(f"{'status':<6} {'shard':<24} {'compile_ms':>10}  {'lines':>5}")
    for name in SHARDS:
        src = ROOT / "self_host" / name
        if not src.is_file():
            print(f"MISSING {name}")
            continue
        text = src.read_text(encoding="utf-8")
        (ROOT / "_profile_src.kab").write_text(text, encoding="utf-8", newline="\n")
        probe = f"""import "self_host/compile"
os_mount("/proj", "{MANIFEST}")
let t0 = date_now_ms()
compile(read_text_file("/proj/_profile_src.kab"))
let t1 = date_now_ms()
println("PROFILE compile_total_ms " + ("" + (t1 - t0)))
return 0
"""
        probe_path.write_text(probe, encoding="utf-8", newline="\n")
        t0 = time.time()
        try:
            r = subprocess.run(
                [str(KAB), "run", str(probe_path)],
                cwd=ROOT,
                capture_output=True,
                text=True,
                env=env,
                timeout=600,
            )
        except subprocess.TimeoutExpired:
            print(f"TIMEOUT {name}")
            continue
        out = (r.stdout or "") + (r.stderr or "")
        ms = "?"
        for line in out.splitlines():
            if "compile_total_ms" in line:
                ms = line.split()[-1]
        ok = r.returncode == 0 and "Error:" not in out
        over = ""
        try:
            if float(ms) > 10000:
                over = " ***"
        except ValueError:
            pass
        lines = len(text.splitlines())
        print(f"{'OK' if ok else 'FAIL':<6} {name:<24} {ms:>10} ms  {lines:>5}{over}")


if __name__ == "__main__":
    main()
