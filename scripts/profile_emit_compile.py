#!/usr/bin/env python3
"""Profile self-hosted compile pipeline: parse / emit / serialize phases.

Usage:
  python scripts/profile_emit_compile.py phases              # emit.kab phases (Kabootar date_now_ms)
  python scripts/profile_emit_compile.py phases parser.kab
  python scripts/profile_emit_compile.py compile             # wall-time compile() only
  python scripts/profile_emit_compile.py bisect emit         # prefix timing (emit.kab)
  python scripts/profile_emit_compile.py bisect parser
  python scripts/profile_emit_compile.py compare             # lexer / parser / emit summary

Output lines tagged PROFILE are parseable; wall-clock printed to stderr.
"""
from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
MANIFEST = ROOT.replace("\\", "/")


def kabootar_bin() -> str:
    for sub in ("target-alt3", "target-alt2", "target"):
        path = os.path.join(ROOT, sub, "debug", "kabootar.exe")
        if os.path.isfile(path):
            return path
    return os.path.join(ROOT, "target", "debug", "kabootar.exe")


KAB = kabootar_bin()
PROFILE_SRC = os.path.join(ROOT, "_profile_src.kab")
PROBE = os.path.join(ROOT, "self_host", "_profile_probe_gen.kab")

PHASES_PROBE = """\
import "self_host/parse"
import "self_host/emit"
import "self_host/serialize"
os_mount("/proj", {manifest})
let src = read_text_file("/proj/_profile_src.kab")
let t0 = date_now_ms()
let ast = parse(src)
let t1 = date_now_ms()
let ir = emit(ast)
let t2 = date_now_ms()
let kbc = serialize_bc(ir)
let t3 = date_now_ms()
println("PROFILE phase parse_ms " + ("" + (t1 - t0)))
println("PROFILE phase emit_ms " + ("" + (t2 - t1)))
println("PROFILE phase serialize_ms " + ("" + (t3 - t2)))
println("PROFILE phase total_ms " + ("" + (t3 - t0)))
println("PROFILE meta kbc_len " + ("" + len(kbc)))
if ast["body"] != undefined {{
    println("PROFILE meta body_stmts " + ("" + len(ast["body"])))
}}
return 0
"""

COMPILE_PROBE = """\
import "self_host/compile"
os_mount("/proj", {manifest})
let src = read_text_file("/proj/_profile_src.kab")
let t0 = date_now_ms()
let kbc = compile(src)
let t1 = date_now_ms()
write_text_file("/proj/{out_name}", kbc)
println("PROFILE compile_total_ms " + ("" + (t1 - t0)))
println("PROFILE meta kbc_len " + ("" + len(kbc)))
return 0
"""

EMIT_STUB = (
    '\npub fn emit(program) { return { "constants": [], "globals": [], '
    '"ops": [], "functions": [], "exports": [], "imports": [] } }\n'
)
PARSER_STUB = (
    '\npub fn parseTokens(tokens) { return { "kind": "Program", "body": [], "imports": [] } }\n'
)


def module_path(name: str) -> str:
    if os.path.isfile(name):
        return os.path.abspath(name)
    path = os.path.join(ROOT, "self_host", name)
    if not os.path.isfile(path):
        raise FileNotFoundError(f"module not found: {name}")
    return path


def wrap_emit_prefix(lines: list[str]) -> str:
    text = "\n".join(lines)
    need = text.count("{") - text.count("}")
    if need > 0:
        text += "\n" + ("}" * need)
    if "pub fn emit" not in text:
        text += EMIT_STUB
    return text + "\n"


def wrap_parser_prefix(lines: list[str]) -> str:
    text = "\n".join(lines)
    need = text.count("{") - text.count("}")
    if need > 0:
        text += "\n" + ("}" * need)
    if "pub fn parseTokens" not in text:
        text += PARSER_STUB
    return text + "\n"


def parse_profile_lines(stdout: str) -> dict[str, float]:
    out: dict[str, float] = {}
    for line in stdout.splitlines():
        m = re.match(r"PROFILE\s+(\S+)\s+(\S+)\s+(-?\d+(?:\.\d+)?)", line)
        if m:
            out[f"{m.group(1)}.{m.group(2)}"] = float(m.group(3))
    return out


def run_probe(probe_src: str, timeout_s: int) -> tuple[bool, dict[str, float], float, str]:
    with open(PROBE, "w", encoding="utf-8", newline="\n") as f:
        f.write(probe_src)
    t0 = time.time()
    try:
        r = subprocess.run(
            [KAB, PROBE],
            cwd=ROOT,
            capture_output=True,
            text=True,
            timeout=timeout_s,
        )
    except subprocess.TimeoutExpired:
        return False, {}, time.time() - t0, f"TIMEOUT after {timeout_s}s"
    wall = time.time() - t0
    combined = (r.stdout or "") + "\n" + (r.stderr or "")
    prof = parse_profile_lines(combined)
    if r.returncode != 0 or "Error:" in combined:
        tail = combined.strip().splitlines()[-1] if combined.strip() else "unknown error"
        return False, prof, wall, tail[:400]
    return True, prof, wall, ""


def copy_source(module: str) -> int:
    path = module_path(module)
    with open(path, encoding="utf-8") as f:
        text = f.read()
    with open(PROFILE_SRC, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    return len(text.splitlines())


def print_phases(prof: dict[str, float], wall: float, label: str) -> None:
    print(f"\n=== {label} ===")
    print(f"  wall_clock_s: {wall:.1f}")
    for key in (
        "phase.parse_ms",
        "phase.emit_ms",
        "phase.serialize_ms",
        "phase.total_ms",
        "compile_total_ms",
        "meta.kbc_len",
        "meta.body_stmts",
    ):
        if key in prof:
            val = prof[key]
            if key.endswith("_ms"):
                print(f"  {key}: {val:.0f} ms ({val / 1000:.1f} s)")
            else:
                print(f"  {key}: {val:.0f}")
    if "phase.total_ms" in prof and prof["phase.total_ms"] > 0:
        emit_pct = 100.0 * prof.get("phase.emit_ms", 0) / prof["phase.total_ms"]
        parse_pct = 100.0 * prof.get("phase.parse_ms", 0) / prof["phase.total_ms"]
        ser_pct = 100.0 * prof.get("phase.serialize_ms", 0) / prof["phase.total_ms"]
        print(f"  share: parse {parse_pct:.1f}% | emit {emit_pct:.1f}% | serialize {ser_pct:.1f}%")


def cmd_phases(module: str, timeout_s: int) -> int:
    n = copy_source(module)
    print(f"profiling phases: {module} ({n} lines)", file=sys.stderr)
    probe = PHASES_PROBE.format(manifest=f'"{MANIFEST}"')
    ok, prof, wall, err = run_probe(probe, timeout_s)
    if not ok:
        print(f"FAIL ({wall:.0f}s): {err}", file=sys.stderr)
        return 1
    print_phases(prof, wall, f"phases {module}")
    return 0


def cmd_compile(module: str, timeout_s: int) -> int:
    n = copy_source(module)
    print(f"profiling compile(): {module} ({n} lines)", file=sys.stderr)
    base = os.path.basename(module).replace(".kab", "")
    out_name = f"_{base}_full_out.kbc"
    probe = COMPILE_PROBE.format(manifest=f'"{MANIFEST}"', out_name=out_name)
    ok, prof, wall, err = run_probe(probe, timeout_s)
    if not ok:
        print(f"FAIL ({wall:.0f}s): {err}", file=sys.stderr)
        return 1
    print_phases(prof, wall, f"compile() {module}")
    return 0


def cmd_bisect(which: str, timeout_scale: float) -> int:
    if which == "emit":
        path = os.path.join(ROOT, "self_host", "emit.kab")
        wrap = wrap_emit_prefix
    elif which == "parser":
        path = os.path.join(ROOT, "self_host", "parser.kab")
        wrap = wrap_parser_prefix
    else:
        print(f"unknown bisect target: {which}", file=sys.stderr)
        return 1

    lines = open(path, encoding="utf-8").read().splitlines()
    n = len(lines)
    boundaries = [50, 100, 150, 200, 250, 300, 400, 500, 600, 700, n]
    boundaries = sorted({b for b in boundaries if b <= n})

    print(f"\n=== bisect timing {which}.kab ({n} lines) ===")
    print(f"{'lines':>6}  {'status':>6}  {'wall_s':>8}  {'emit_ms':>10}  {'total_ms':>10}  note")
    for b in boundaries:
        src = wrap(lines[:b])
        with open(PROFILE_SRC, "w", encoding="utf-8", newline="\n") as f:
            f.write(src)
        timeout = max(120, int(b * timeout_scale))
        probe = PHASES_PROBE.format(manifest=f'"{MANIFEST}"')
        ok, prof, wall, err = run_probe(probe, timeout)
        emit_ms = prof.get("phase.emit_ms", 0)
        total_ms = prof.get("phase.total_ms", 0)
        status = "OK" if ok else "FAIL"
        note = "" if ok else err[:60]
        print(
            f"{b:>6}  {status:>6}  {wall:8.1f}  {emit_ms:10.0f}  {total_ms:10.0f}  {note}"
        )
    return 0


def cmd_compare(timeout_s: int) -> int:
    modules = ("lexer.kab", "parser.kab", "emit.kab")
    print("\n=== compare modules (phases) ===")
    print(f"{'module':<14} {'lines':>6}  {'parse_s':>8}  {'emit_s':>8}  {'ser_s':>8}  {'total_s':>8}")
    for mod in modules:
        try:
            n = copy_source(mod)
        except FileNotFoundError:
            continue
        probe = PHASES_PROBE.format(manifest=f'"{MANIFEST}"')
        ok, prof, _wall, err = run_probe(probe, timeout_s)
        if not ok:
            print(f"{mod:<14} {n:>6}  FAIL: {err[:50]}")
            continue
        parse_s = prof.get("phase.parse_ms", 0) / 1000
        emit_s = prof.get("phase.emit_ms", 0) / 1000
        ser_s = prof.get("phase.serialize_ms", 0) / 1000
        total_s = prof.get("phase.total_ms", 0) / 1000
        print(f"{mod:<14} {n:>6}  {parse_s:8.1f}  {emit_s:8.1f}  {ser_s:8.1f}  {total_s:8.1f}")
    return 0


def main() -> int:
    p = argparse.ArgumentParser(description="Profile self-hosted compile pipeline")
    p.add_argument(
        "command",
        choices=("phases", "compile", "bisect", "compare"),
        help="phases=parse/emit/serialize; compile=compile(); bisect=prefix timing",
    )
    p.add_argument(
        "target",
        nargs="?",
        default="emit.kab",
        help="module for phases/compile/bisect (default emit.kab)",
    )
    p.add_argument(
        "--timeout",
        type=int,
        default=0,
        help="subprocess timeout seconds (0=auto)",
    )
    p.add_argument(
        "--bisect-scale",
        type=float,
        default=8.0,
        help="seconds per line for bisect timeout estimate",
    )
    args = p.parse_args()

    if not os.path.isfile(KAB):
        print(f"kabootar not found: {KAB}", file=sys.stderr)
        print("build: CARGO_TARGET_DIR=target-alt3 cargo build --bin kabootar", file=sys.stderr)
        return 1

    if args.command == "phases":
        timeout = args.timeout or max(600, int(copy_source(args.target) * 20))
        return cmd_phases(args.target, timeout)
    if args.command == "compile":
        timeout = args.timeout or max(600, int(copy_source(args.target) * 25))
        return cmd_compile(args.target, timeout)
    if args.command == "bisect":
        return cmd_bisect(args.target.replace(".kab", ""), args.bisect_scale)
    if args.command == "compare":
        timeout = args.timeout or 7200
        return cmd_compare(timeout)
    return 1


if __name__ == "__main__":
    sys.exit(main())
