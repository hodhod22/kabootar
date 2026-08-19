"""SH6: inline self_host vm_* shards into a few dispatch/session modules."""
from __future__ import annotations

import re
import sys
from collections import defaultdict, deque
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "self_host"
IMP_RE = re.compile(r'^(?:pub\s+)?import\s+"self_host/([^"]+)"\s*$')


def stem(path: Path) -> str:
    return path.name[: -len(".kab")] if path.name.endswith(".kab") else path.name


def parse_file(path: Path) -> tuple[list[str], str]:
    text = path.read_text(encoding="utf-8")
    imports: list[str] = []
    body_lines: list[str] = []
    for line in text.splitlines(True):
        m = IMP_RE.match(line.strip())
        if m:
            imports.append(m.group(1))
        else:
            body_lines.append(line)
    return imports, "".join(body_lines).strip() + "\n"


def topo(members: set[str], edges: dict[str, set[str]]) -> list[str]:
    indeg = {m: 0 for m in members}
    for m in members:
        for d in edges[m]:
            if d in members:
                indeg[m] += 1
    q = deque(sorted(m for m, n in indeg.items() if n == 0))
    out: list[str] = []
    while q:
        n = q.popleft()
        out.append(n)
        for m in members:
            if n in edges[m]:
                indeg[m] -= 1
                if indeg[m] == 0:
                    q.append(m)
    if len(out) != len(members):
        rest = sorted(members - set(out))
        out.extend(rest)
    return out


def merge_group(target: str, extra_globs: list[str]) -> int:
    files = {stem(p): p for p in ROOT.glob(f"{target}.kab")}
    for g in extra_globs:
        for p in ROOT.glob(g):
            files[stem(p)] = p
    if target not in files:
        raise SystemExit(f"missing target {target}.kab")
    members = set(files)
    edges: dict[str, set[str]] = defaultdict(set)
    parsed = {}
    for name, path in files.items():
        imps, body = parse_file(path)
        parsed[name] = (imps, body)
        edges[name] = {i for i in imps if i in members}
    order = topo(members, edges)
    ext: list[str] = []
    seen = set()
    bodies: list[str] = []
    lets_seen: set[str] = set()
    for name in order:
        imps, body = parsed[name]
        for i in imps:
            if i in members or i in seen:
                continue
            seen.add(i)
            ext.append(i)
        kept = []
        for line in body.splitlines(True):
            s = line.strip()
            # Only drop duplicate *module-level* lets (indent = 0). Function
            # locals like `let st =` are reused in every op handler.
            if line[:1] not in " \t" and s.startswith("let ") and "=" in s:
                key = s.split("=")[0].strip()
                if key in lets_seen:
                    continue
                lets_seen.add(key)
            kept.append(line)
        chunk = "".join(kept).strip()
        if chunk:
            bodies.append(f"// --- {name} ---\n{chunk}\n")
    header = "".join(f'import "self_host/{i}"\n' for i in ext)
    out = header + ("\n" if header else "") + "\n".join(bodies)
    files[target].write_text(out.rstrip() + "\n", encoding="utf-8")
    removed = 0
    for name, path in files.items():
        if name == target:
            continue
        path.unlink()
        removed += 1
    for p in ROOT.glob("*.kab"):
        text = p.read_text(encoding="utf-8")
        orig = text
        for name in members:
            if name == target:
                continue
            text = text.replace(f'import "self_host/{name}"', f'import "self_host/{target}"')
            text = text.replace(f'pub import "self_host/{name}"', f'pub import "self_host/{target}"')
        if text != orig:
            lines = text.splitlines(True)
            seen_imp = set()
            new = []
            for line in lines:
                m = IMP_RE.match(line.strip())
                if m:
                    if m.group(1) in seen_imp:
                        continue
                    seen_imp.add(m.group(1))
                new.append(line)
            p.write_text("".join(new), encoding="utf-8")
    print(f"merged {len(members)} -> {target}.kab (removed {removed})")
    return removed


def main() -> None:
    groups = [
        ("vm_ops_arith_a", ["vm_ops_arith_*.kab"]),
        ("vm_ops_cmp_a", ["vm_ops_cmp_*.kab"]),
        ("vm_ops_data_a", ["vm_ops_data_*.kab"]),
        ("vm_ops_mem_a", ["vm_ops_mem_*.kab"]),
        ("vm_ops_jump_a", ["vm_ops_jump_*.kab"]),
        ("vm_ops_ctrl_a", ["vm_ops_ctrl_*.kab"]),
        ("vm_s_stack", ["vm_s_*.kab"]),
        ("vm_run_dispatch_plain", ["vm_run_dispatch_*.kab"]),
        ("vm_run_norm_bc", ["vm_run_norm_*.kab"]),
        ("vm_run_prep_fn", ["vm_run_prep_*.kab"]),
        (
            "vm_run_call",
            [
                "vm_run_call*.kab",
                "vm_run_callee*.kab",
                "vm_run_args_*.kab",
            ],
        ),
        ("vm_run_new_run", ["vm_run_new_*.kab"]),
        (
            "vm_run_restore_mod",
            ["vm_run_restore_*.kab", "vm_run_save_*.kab"],
        ),
        ("vm_run_ops_loop", ["vm_run_ops_*.kab"]),
        (
            "vm_run_session",
            [
                "vm_run_bind_*.kab",
                "vm_run_alloc_*.kab",
                "vm_run_fill_*.kab",
                "vm_run_reset.kab",
                "vm_run_wire.kab",
                "vm_run_hook_*.kab",
                "vm_run_fn_body.kab",
                "vm_run_meth_body.kab",
                "vm_run_this_frame.kab",
                "vm_run_take_result.kab",
            ],
        ),
        ("vm_run_tramp", ["vm_run_tramp*.kab"]),
        ("vm_run_mod_run", ["vm_run_mod_*.kab"]),
    ]
    n = 0
    for target, globs in groups:
        n += merge_group(target, globs)
    leftover = sorted(
        p.name
        for p in ROOT.glob("vm*.kab")
        if p.name.startswith("vm_") or p.name == "vm.kab"
    )
    print("total removed", n)
    print("vm leftover", len(leftover))
    for name in leftover:
        print(" ", name)


if __name__ == "__main__":
    sys.exit(main())
