"""SH5: inline a set of self_host shards into one target module."""
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
            if s.startswith("let ") and "=" in s:
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
    # rewrite remaining .kab imports
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
        ("parser_stmt", ["parser_stmt_*.kab"]),
        ("parser_postfix", ["parser_postfix_*.kab"]),
        ("parser_compare", ["parser_compare_*.kab"]),
        ("parser_add_shift", ["parser_add_shift_*.kab"]),
        ("lexer_scan", ["lexer_scan_*.kab"]),
        ("emit_stmt_body", ["emit_stmt_*.kab", "emit_if_stmt.kab"]),
        ("emit_expr_body", ["emit_expr_*.kab"]),
        ("emit_type_infer_args", ["emit_type_infer_*.kab"]),
        ("emit_main_init", ["emit_main_init_*.kab"]),
        ("serialize_acc", ["serialize_acc_*.kab"]),
        ("serialize_out", ["serialize_out_*.kab"]),
        ("serialize_fns", ["serialize_fn_*.kab"]),
        ("serialize_arrows", ["serialize_arrow_*.kab"]),
        ("serialize_classes", ["serialize_class_*.kab"]),
    ]
    n = 0
    for target, globs in groups:
        n += merge_group(target, globs)
    print("total removed", n)


if __name__ == "__main__":
    sys.exit(main())
