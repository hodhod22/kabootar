#!/usr/bin/env python3
"""Calibrate self-host compile wall time for small sources."""
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


def compile_src(text: str, label: str, timeout: int = 180) -> None:
    (ROOT / "_profile_src.kab").write_text(text, encoding="utf-8", newline="\n")
    probe = f"""import "self_host/compile"
os_mount("/proj", "{MANIFEST}")
let t0 = date_now_ms()
let kbc = compile(read_text_file("/proj/_profile_src.kab"))
let t1 = date_now_ms()
println("PROFILE compile_total_ms " + ("" + (t1 - t0)))
return 0
"""
    (ROOT / "self_host/_profile_probe_gen.kab").write_text(probe, encoding="utf-8", newline="\n")
    env = dict(os.environ)
    env["KABOOTAR_VM"] = "host"
    env["KABOOTAR_COMPILE"] = "rust"
    t0 = time.time()
    r = subprocess.run(
        [str(KAB), "run", "self_host/_profile_probe_gen.kab"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        env=env,
        timeout=timeout,
    )
    wall = time.time() - t0
    out = (r.stdout or "") + (r.stderr or "")
    ms = "?"
    for line in out.splitlines():
        if "compile_total_ms" in line:
            ms = line.split()[-1]
    ok = r.returncode == 0 and "Error:" not in out
    print(f"{label} wall={wall:.1f}s ms={ms} ok={ok}", flush=True)
    if not ok:
        print(" ", (out.strip().splitlines() or ["?"])[-1][:200], flush=True)


def main() -> None:
    compile_src("pub fn foo(x) { return x + 1 }\n", "tiny")
    lines = ['import "self_host/emit_defs"\n', "let eCallRet = 0\n"]
    for i in range(20):
        lines.append(f"fn f{i}(E, a) {{\n    E[\"x\"] = a\n    return a\n}}\n")
    lines.append("pub fn entry(E) { return f0(E, 1) }\n")
    compile_src("".join(lines), "synth20fns")
    compile_src(
        (ROOT / "self_host/serialize_const.kab").read_text(encoding="utf-8"),
        "serialize_const",
    )
    # First ~80 lines of emit_main (imports + a few fns)
    em = (ROOT / "self_host/emit_main.kab").read_text(encoding="utf-8").splitlines(True)
    body = "".join(em[:80])
    need = body.count("{") - body.count("}")
    if need > 0:
        body += "\n" + ("}" * need)
    body += "\npub fn emitMain(E, program){return {}}\n"
    compile_src(body, "emit_main_80")


if __name__ == "__main__":
    main()
