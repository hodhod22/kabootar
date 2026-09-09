#!/usr/bin/env python3
"""Warm-measure previously OVER vm shards after emit_defs strip."""
import os, subprocess, pathlib

binp = os.environ["KABOOTAR_BIN"]
mount = "c:/after-2026-06-03/new-kabootar-reserv/nova-interpreter"

# Former OVER + new 1-op leaves
files = [
    "self_host/vm_s_push.kab",
    "self_host/vm_s_pop.kab",
    "self_host/vm_s_ip.kab",
    "self_host/vm_s_this_is.kab",
    "self_host/vm_s_this_is_obj.kab",
    "self_host/vm_s_catch.kab",
    "self_host/vm_ops_ctrl_throw.kab",
    "self_host/vm_ops_ctrl_return_ret.kab",
    "self_host/vm_ops_arith_add.kab",
    "self_host/vm_ops_arith_mul.kab",
    "self_host/vm_ops_arith_bit_and.kab",
    "self_host/vm_ops_arith_mod.kab",
    "self_host/vm_ops_arith_shift.kab",
    "self_host/vm_ops_arith_shl.kab",
    "self_host/vm_ops_cmp_and.kab",
    "self_host/vm_ops_cmp_eq_only.kab",
    "self_host/vm_ops_cmp_lt.kab",
    "self_host/vm_ops_cmp_not.kab",
    "self_host/vm_ops_cmp_member.kab",
    "self_host/vm_ops_cmp_logic.kab",
    "self_host/vm_ops_data_make_object.kab",
    "self_host/vm_ops_data_make_array.kab",
    "self_host/vm_ops_data_index_set.kab",
    "self_host/vm_ops_data_set_member.kab",
    "self_host/vm_ops_data_len.kab",
    "self_host/vm_ops_data_concat.kab",
    "self_host/vm_ops_data_dup.kab",
    "self_host/vm_ops_data_merge.kab",
    "self_host/vm_ops_data_option_some.kab",
    "self_host/vm_ops_jump_result.kab",
    "self_host/vm_ops_jump_if_true.kab",
    "self_host/vm_ops_mem_acc_add_local.kab",
    "self_host/vm_ops_mem_const.kab",
    "self_host/vm_ops_mem_load_local.kab",
    "self_host/vm_ops_mem_array_push_local.kab",
    "self_host/vm_ops_mem_index_get.kab",
    "self_host/vm_ops_mem_global.kab",
    "self_host/vm_ops_mem_take.kab",
    "self_host/vm_s_member_get.kab",
]

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
for f in files:
    lines.append(f'tryComp("{f}")')

pathlib.Path("self_host/_try_comp.kab").write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
subprocess.run([binp, "run", "self_host/_try_comp.kab"], check=False)
