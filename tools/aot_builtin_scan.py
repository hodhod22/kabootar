#!/usr/bin/env python3
"""AOT-0b builtin-coverage scan: parse a bytecode_host_module_record JSON dump
and report every `load_global`+`call` callee that is neither a module fn nor
an inline-lowered builtin -> the AOT-5 host-trampoline worklist.
Usage: aot_builtin_scan.py <bc.json>
"""
import json
import sys

COVERED = {
    "len", "push", "pop", "slice", "concat", "typeof", "str_slice",
    "substring", "char_at", "char_code_at", "str_at", "from_char_code",
    "from_code_point", "parse_int", "abs", "min", "max", "floor", "ceil",
    "round", "is_null", "is_undefined", "is_bool", "is_number", "is_nan",
    "is_string", "is_array", "array_is_array", "is_object", "is_function",
    "object_has_own", "has_key", "object_array_push", "object_array_pop",
    "object_array_clear", "object_array_truncate", "str_index_of",
    "str_includes", "replace_all", "comptime_fold", "is_float", "env_get",
    "date_now_ms", "json_stringify", "instanceof",
}


def scan(ops, globals_, fn_syms, uncovered, dyn):
    i = 0
    while i < len(ops):
        o = ops[i]
        if isinstance(o, dict) and o.get("op") == "load_global" and i + 1 < len(ops):
            nxt = ops[i + 1]
            if isinstance(nxt, dict) and nxt.get("op") == "call":
                arg = o.get("arg", 0)
                nm = globals_[arg] if isinstance(arg, int) and arg < len(globals_) else f"<gi{arg}>"
                if nm not in fn_syms and nm not in COVERED:
                    uncovered.add(nm)
                i += 2
                continue
        if isinstance(o, dict) and o.get("op") == "call":
            dyn[0] += 1
        i += 1


def main(path):
    # first line = json_stringify output; the runner may append "=> ..." lines
    line = open(path, encoding="utf-8").readline()
    bc = json.loads(line)
    globals_ = bc.get("globals", [])
    fn_syms = {f.get("sym") for f in bc.get("functions", [])}
    uncovered, dyn = set(), [0]
    scan(bc.get("ops", []), globals_, fn_syms, uncovered, dyn)
    op_names = {}
    for f in bc.get("functions", []):
        scan(f.get("ops", []), globals_, fn_syms, uncovered, dyn)
        for a in f.get("arrows", []):
            scan(a.get("ops", []), globals_, fn_syms, uncovered, dyn)
        for o in f.get("ops", []):
            if isinstance(o, dict):
                op_names[o["op"]] = op_names.get(o["op"], 0) + 1
    print(f"{path}: {len(uncovered)} uncovered callee(s), {dyn[0]} dynamic call(s), {len(fn_syms)} fns")
    for n in sorted(uncovered):
        print(f"  callv {n}")


if __name__ == "__main__":
    main(sys.argv[1])
