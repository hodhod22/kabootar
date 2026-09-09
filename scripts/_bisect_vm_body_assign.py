#!/usr/bin/env python3
import os, subprocess, pathlib

binp = os.environ["KABOOTAR_BIN"]
root = pathlib.Path(".").resolve()
mount = str(root).replace("\\", "/")
if len(mount) >= 2 and mount[1] == ":":
    mount = mount[0].lower() + mount[1:]

src = pathlib.Path("self_host/vm_run_body.kab").read_text(encoding="utf-8")
lines = src.splitlines()
ends = []
depth = 0
in_fn = False
for i, line in enumerate(lines, 1):
    st = line.lstrip()
    if depth == 0 and (st.startswith("fn ") or st.startswith("pub fn ")):
        in_fn = True
    depth += line.count("{") - line.count("}")
    if in_fn and depth == 0:
        ends.append(i)
        in_fn = False
print("fn ends", ends)


def try_prefix(nlines, label):
    text = "\n".join(lines[:nlines]) + "\n"
    need = text.count("{") - text.count("}")
    if need > 0:
        text += "\n" + ("}" * need) + "\n"
    pathlib.Path("self_host/_body_prefix.kab").write_text(text, encoding="utf-8", newline="\n")
    probe = f"""import "self_host/compile"
os_mount("/proj", "{mount}")
try {{
  let s = read_text_file("/proj/self_host/_body_prefix.kab")
  compile(s)
  println("OK {label} lines={nlines}")
}} catch (e) {{
  println("ERR {label} lines={nlines} " + e)
}}
"""
    pathlib.Path("self_host/_body_bisect.kab").write_text(probe, encoding="utf-8", newline="\n")
    r = subprocess.run(
        [binp, "run", "self_host/_body_bisect.kab"],
        env={**os.environ},
        capture_output=True,
        text=True,
        timeout=180,
    )
    out = (r.stdout or "") + (r.stderr or "")
    for line in out.splitlines():
        if line.startswith("OK ") or line.startswith("ERR "):
            print(line)
            return line.startswith("OK ")
    print("NO_MATCH", out[-300:])
    return False


last_ok = None
for e in ends:
    ok = try_prefix(e, f"end{e}")
    if ok:
        last_ok = e
    else:
        print("FAIL at end", e, "last_ok", last_ok)
        if last_ok is not None:
            # line-level bisect
            lo, hi = last_ok, e
            while hi - lo > 1:
                mid = (lo + hi) // 2
                if try_prefix(mid, f"mid{mid}"):
                    lo = mid
                else:
                    hi = mid
            print("first bad around line", hi)
            for i in range(max(1, hi - 5), min(len(lines), hi + 3) + 1):
                print(f"{i}: {lines[i-1]}")
        break
else:
    print("all prefixes OK — failure may be whole-file only")
