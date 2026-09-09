#!/usr/bin/env python3
"""Split one emit .kab shard: extract large top-level if-blocks into helper modules."""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SH = ROOT / "self_host"


def find_pub_fn(lines: list[str]) -> tuple[str, int, int, int]:
    fn_i = next(i for i, l in enumerate(lines) if l.startswith("pub fn "))
    m = re.match(r"pub fn (\w+)", lines[fn_i])
    assert m
    name = m.group(1)
    depth = 0
    body0 = None
    fn_end = None
    for i in range(fn_i, len(lines)):
        depth += lines[i].count("{") - lines[i].count("}")
        if body0 is None and "{" in lines[i]:
            body0 = i + 1
        if body0 is not None and depth == 0:
            fn_end = i
            break
    assert body0 is not None and fn_end is not None
    return name, fn_i, body0, fn_end


def find_large_ifs(body_lines: list[str], max_lines: int) -> list[tuple[int, int]]:
    depth = 0
    found: list[tuple[int, int]] = []
    i = 0
    while i < len(body_lines):
        line = body_lines[i]
        stripped = line.strip()
        if stripped.startswith("if ") and stripped.endswith("{") and depth == 0:
            start = i
            sd = depth
            depth += line.count("{") - line.count("}")
            i += 1
            while i < len(body_lines) and depth > sd:
                depth += body_lines[i].count("{") - body_lines[i].count("}")
                i += 1
            if (i - 1) - start + 1 >= max_lines:
                found.append((start, i - 1))
            continue
        depth += line.count("{") - line.count("}")
        i += 1
    return found


def dedent(lines: list[str], n: int = 4) -> list[str]:
    return [ln[n:] if ln.startswith(" " * n) else ln for ln in lines]


def rewrite_done(body: str, done_key: str | None, handler_returns_bool: bool) -> str:
    if not done_key:
        return body
    out = []
    for line in body.splitlines(True):
        if re.match(r"^[ \t]*return true[ \t]*$", line):
            ind = line[: len(line) - len(line.lstrip())]
            out.append(f'{ind}E["{done_key}"] = 1\n')
            if handler_returns_bool:
                out.append(f"{ind}return true\n")
            else:
                out.append(f"{ind}return 0\n")
        elif re.match(r"^[ \t]*return[ \t]*$", line):
            ind = line[: len(line) - len(line.lstrip())]
            out.append(f"{ind}return 0\n")
        else:
            out.append(line)
    return "".join(out)


def split_shard(
    path: Path,
    max_lines: int,
    done_key: str | None,
    counter: list[int],
) -> list[str]:
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines(True)
    fn_name, fn_i, body0, fn_end = find_pub_fn(lines)
    header = "".join(lines[:fn_i])
    body = lines[body0:fn_end]

    guard: list[str] = []
    work = list(body)
    if work and "return false" in work[0]:
        d = 0
        g = 0
        d += work[0].count("{") - work[0].count("}")
        g = 1
        while g < len(work) and d > 0:
            d += work[g].count("{") - work[g].count("}")
            g += 1
        guard = work[:g]
        work = work[g:]

    while work and work[-1].strip() in ("return true", ""):
        if work[-1].strip() == "return true":
            work = work[:-1]
            break
        work = work[:-1]

    handler_bool = fn_name.startswith("emitExpr_") or fn_name.startswith("emitStmt_")
    extra_imports: list[str] = re.findall(r'import "self_host/[^"]+"', header)
    written: list[str] = []

    def extract(body_lines: list[str], parent_stem: str) -> list[str]:
        nonlocal written
        body = list(body_lines)
        for start, end in sorted(find_large_ifs(body, max_lines), key=lambda x: -x[0]):
            block = body[start : end + 1]
            cond_m = re.match(r"(\s*)if (.+) \{", block[0].rstrip("\n"))
            if not cond_m:
                continue
            indent, cond = cond_m.group(1), cond_m.group(2)
            inner = dedent(block[1:-1])
            counter[0] += 1
            hname = f"{fn_name}_s{counter[0]}"
            helper_file = f"{path.stem}_s{counter[0]}.kab"
            inner_new = extract(inner, helper_file[:-4])
            imp_lines = list(dict.fromkeys(extra_imports))
            for w in written:
                mod = Path(w).stem
                imp = f'import "self_host/{mod}"'
                if imp not in imp_lines:
                    imp_lines.append(imp)
            htext = "\n".join(imp_lines) + "\n\n"
            lets = [ln for ln in header.splitlines() if ln.startswith("let ")]
            if lets:
                htext += "\n".join(lets) + "\n\n"
            htext += f"pub fn {hname}(E) {{\n"
            htext += rewrite_done("".join(inner_new), done_key, False)
            htext += "}\n"
            (SH / helper_file).write_text(htext, encoding="utf-8", newline="\n")
            written.append(helper_file)
            print(f"  wrote {helper_file} ({len(htext.splitlines())} lines)")
            imp = f'import "self_host/{Path(helper_file).stem}"'
            if imp not in extra_imports:
                extra_imports.append(imp)
            if done_key:
                repl = [
                    f"{indent}if {cond} {{\n",
                    f"{indent}    {hname}(E)\n",
                    f'{indent}    if E["{done_key}"] == 1 {{\n',
                    f"{indent}        return true\n" if handler_bool else f"{indent}        return 0\n",
                    f"{indent}    }}\n",
                    f"{indent}}}\n",
                ]
            else:
                repl = [
                    f"{indent}if {cond} {{\n",
                    f"{indent}    {hname}(E)\n",
                    f"{indent}}}\n",
                ]
            body = body[:start] + repl + body[end + 1 :]
        return body

    new_work = extract(work, path.stem)
    tail = "    return true\n" if handler_bool else ""
    new_fn = (
        f"pub fn {fn_name}(E) {{\n"
        + "".join(guard)
        + rewrite_done("".join(new_work), done_key, handler_bool)
        + tail
        + "}\n"
    )
    imp_lines = list(dict.fromkeys(extra_imports))
    lets = [ln for ln in header.splitlines() if ln.startswith("let ")]
    out = "\n".join(imp_lines) + "\n\n"
    if lets:
        out += "\n".join(lets) + "\n\n"
    out += new_fn
    path.write_text(out, encoding="utf-8", newline="\n")
    print(f"updated {path.name} ({len(out.splitlines())} lines)")
    return written


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("path", help="shard under self_host/, e.g. emit_expr_call_h2.kab")
    ap.add_argument("--max", type=int, default=35)
    ap.add_argument("--done-key", default=None)
    args = ap.parse_args()
    path = Path(args.path)
    if not path.exists():
        path = SH / path.name
    if not path.exists():
        sys.exit(f"not found: {args.path}")
    counter = [0]
    split_shard(path, args.max, args.done_key, counter)
    print(f"helpers: {counter[0]}")


if __name__ == "__main__":
    main()
