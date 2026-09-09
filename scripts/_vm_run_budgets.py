#!/usr/bin/env python3
from pathlib import Path
import os, subprocess

binp = os.environ["KABOOTAR_BIN"]
mount = "c:/after-2026-06-03/new-kabootar-reserv/nova-interpreter"
files = sorted(Path("self_host").glob("vm_run_*.kab"))
files = [p for p in files if p.name != "vm_run_body.kab"] + [
    Path("self_host/vm_run_body.kab")
]
rels = [str(p).replace("\\", "/") for p in files]
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
    'tryComp("self_host/vm_s_push.kab")',
    'tryComp("self_host/vm_s_push.kab")',
]
for r in rels:
    lines.append(f'tryComp("{r}")')
Path("self_host/_try_comp.kab").write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
print("n", len(rels))
subprocess.run([binp, "run", "self_host/_try_comp.kab"], check=False)
