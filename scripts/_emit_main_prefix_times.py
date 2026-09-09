#!/usr/bin/env python3
"""Time self-host compile of emit_main.kab prefixes at key cuts."""
from __future__ import annotations

import os
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
KAB = Path(os.environ.get("KABOOTAR_BIN", ROOT / "target-p6b9-rel/release/kabootar.exe"))
src = (ROOT / "self_host/emit_main.kab").read_text(encoding="utf-8").splitlines(True)
MANIFEST = str(ROOT).replace("\\", "/")
if len(MANIFEST) >= 2 and MANIFEST[1] == ":":
    MANIFEST = MANIFEST[0].lower() + MANIFEST[1:]


def wrap(n: int) -> str:
    body = "".join(src[:n])
    need = body.count("{") - body.count("}")
    if need > 0:
        body += "\n" + ("}" * need)
    if "pub fn emitMain" not in body:
        body += "\npub fn emitMain(E, program){return {}}\n"
    return body


def time_compile(n: int, label: str, timeout: int = 900) -> None:
    (ROOT / "_profile_src.kab").write_text(wrap(n), encoding="utf-8", newline="\n")
    probe = f"""import "self_host/compile"
os_mount("/proj", "{MANIFEST}")
let t0 = date_now_ms()
let kbc = compile(read_text_file("/proj/_profile_src.kab"))
let t1 = date_now_ms()
println("PROFILE compile_total_ms " + ("" + (t1 - t0)))
println("PROFILE meta kbc_len " + ("" + len(kbc)))
return 0
"""
    (ROOT / "self_host/_profile_probe_gen.kab").write_text(probe, encoding="utf-8", newline="\n")
    env = dict(os.environ)
    env["KABOOTAR_VM"] = "host"
    env["KABOOTAR_COMPILE"] = "rust"
    t0 = time.time()
    try:
        r = subprocess.run(
            [str(KAB), "run", "self_host/_profile_probe_gen.kab"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            env=env,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        print(f"{label} n={n} TIMEOUT", flush=True)
        return
    wall = time.time() - t0
    out = (r.stdout or "") + (r.stderr or "")
    ms = "?"
    for line in out.splitlines():
        if "compile_total_ms" in line:
            ms = line.split()[-1]
    ok = "Error:" not in out and r.returncode == 0
    print(f"{label} n={n} ok={ok} wall={wall:.1f}s compile_ms={ms}", flush=True)
    if not ok:
        last = out.strip().splitlines()[-1] if out.strip() else "?"
        print("  ", last[:200], flush=True)


def main() -> None:
    cuts = [
        (323, "helpers"),
        (700, "expr_mid"),
        (1293, "expr_end"),
        (1358, "if_helpers"),
        (1600, "stmt_mid"),
        (1874, "stmt_end"),
        (1969, "full"),
    ]
    for n, lab in cuts:
        time_compile(n, lab)


if __name__ == "__main__":
    main()
