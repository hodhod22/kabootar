#!/usr/bin/env python3
"""Measure compile wall time for every self_host/parser_*.kab shard."""
from __future__ import annotations

import os
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
KAB = Path(os.environ.get("KABOOTAR_BIN", ROOT / "target-p6b9-rel/release/kabootar.exe"))
MANIFEST = str(ROOT).replace("\\", "/")
if len(MANIFEST) >= 2 and MANIFEST[1] == ":":
    MANIFEST = MANIFEST[0].lower() + MANIFEST[1:]


def main() -> None:
    env = dict(os.environ)
    env["KABOOTAR_VM"] = "host"
    env["KABOOTAR_COMPILE"] = "rust"
    probe_path = ROOT / "self_host/_profile_probe_gen.kab"
    rows: list[tuple[str, float, int, str]] = []

    for src in sorted((ROOT / "self_host").glob("parser_*.kab")):
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
            rows.append((src.name, 999999.0, len(text.splitlines()), "TIMEOUT"))
            continue
        out = (r.stdout or "") + (r.stderr or "")
        ms = "?"
        for line in out.splitlines():
            if "compile_total_ms" in line:
                ms = line.split()[-1]
        try:
            msf = float(ms)
        except ValueError:
            msf = 0.0
        ok = r.returncode == 0 and "Error:" not in out
        rows.append((src.name, msf, len(text.splitlines()), "OK" if ok else "FAIL"))

    rows.sort(key=lambda x: -x[1])
    print(f"{'shard':<32} {'compile_ms':>10}  {'lines':>5}")
    for name, ms, lines, st in rows:
        over = " ***" if ms > 10000 else ""
        print(f"{name:<32} {ms:>10.0f} ms  {lines:>5}{over}  {st}")


if __name__ == "__main__":
    main()
