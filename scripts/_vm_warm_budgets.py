#!/usr/bin/env python3
"""Warm self-host compile timings for vm_* shards."""
import os, subprocess, pathlib

binp = os.environ["KABOOTAR_BIN"]
mount = "c:/after-2026-06-03/new-kabootar-reserv/nova-interpreter"
root = pathlib.Path("self_host")
files = sorted(root.glob("vm_ops_*.kab")) + sorted(root.glob("vm_s_*.kab"))
files = [p for p in files if not p.name.startswith("_")]
# body last
files.append(root / "vm_run_body.kab")

lines = [
    'import "self_host/compile"',
    f'os_mount("/proj", "{mount}")',
    "fn tryComp(rel) {",
    '    let src = read_text_file("/proj/" + rel)',
    "    try {",
    "        let t0 = date_now_ms()",
    "        compile(src)",
    "        let ms = date_now_ms() - t0",
    '        let flag = ""',
    "        if ms > 10000 {",
    '            flag = " OVER"',
    "        }",
    '        println("OK " + rel + " ms=" + ms + flag)',
    "    } catch (e) {",
    '        println("ERR " + rel + " " + e)',
    "    }",
    "}",
]
# warm twice on stack
lines.append('tryComp("self_host/vm_s_stack.kab")')
lines.append('tryComp("self_host/vm_s_stack.kab")')
for p in files:
    rel = str(p).replace("\\", "/")
    lines.append(f'tryComp("{rel}")')

pathlib.Path("self_host/_try_comp.kab").write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
print("files", len(files))
r = subprocess.run([binp, "run", "self_host/_try_comp.kab"], check=False)
