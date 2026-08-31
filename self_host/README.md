# Self-hosted Kabootar compiler

**Slutmål (nolltolerans):** hela stacken är `.kab`. Rust är skuld tills [SH28](../docs/ROADMAP.md#kabootar-på-egna-fötter--noll-rust). Plan: [docs/ROADMAP.md — Kabootar på egna fötter](../docs/ROADMAP.md#kabootar-på-egna-fötter--noll-rust).

Produktkompilatorn är `self_host/compile.kab`. Plan: **[docs/ROADMAP.md — Våg SH](../docs/ROADMAP.md)**. SH6: Kab-VM **kab-only default**; packed `.kbcb` run; **`in`**; **`await`**; **`array_slice_from`**; **`new_instance_from_array`**; self-host **`let [a, ...rest]`** / **`let { a, ...rest }`** / **`let { x: [a, b] }`** / **`[1, ...xs]`** / **`{ ...obj }`** / **`{ a }`** / **`{ foo() {} }`** / **`{ [k]: v }`** / **`is`/`is not`** / **`fn f(a, b = 3)`** / **`fn f(a, ...xs)`** / **`(a, b = 3) =>`** / **`(a, ...xs) =>`** / **`class C { fn add(a, b = 3) }`** / **`fn rest(a, ...xs)`** / **`{ add(a, b = 3) {} }`** / **`{ rest(a, ...xs) {} }`** / **`trait T { fn add(a, b = 3) }`** / **`fn rest(a, ...xs)`** / **`o?.x`** / **`xs?.[0]`** / **`f?.()`** / **`delete o.z`** / **`delete o.a.b`** / **`delete xs[0].x`** / **`delete o[k]`** / **`delete o.items[0].x`** / **`delete xs[0][0].x`** / **`delete this.z`** / **`delete o.items[0][0].x`** / **`delete o.a.b.c`** / **`delete this.a.b`** / **`delete o[k].x`** / **`delete super.z`** / **`delete this[k]`** / **`delete super[k]`** / **`delete super.a.b`** / **`delete o[k][j]`** / **`delete super.a[k]`** / **`delete this[k].x`** / **`delete super[k].x`** / **`delete this.a[k]`** / **`delete o.a[k]`** / **`delete this[k][j]`** / **`delete super[k][j]`** / **`delete o.items[0][k]`** / **`delete this.a.b[k]`** / **`delete o.a.b[k]`** / **`delete xs[0][0][k]`** / **`delete super.a.b[k]`** / **`delete this.items[0][k]`** / **`delete super.items[0][k]`** / **`delete this.items[0][0][k]`** / **`delete o.items[0][0][k]`** / **`delete super.items[0][0][k]`** / **`n &= 3`** / **`n |= 2`** / **`n ^= 3`** / **`o.x &= 3`** / **`xs[0] |= 2`** / **`o.x ^= 3`** / **`this.n &= 3`** / **`xs[0] ^= 3`** / **`super.n |= 2`** / **`o.x |= 2`** / **`xs[0] &= 3`** / **`this.n |= 2`** / **`this.n ^= 3`** / **`super.n &= 3`** / **`super.n ^= 3`** / **`o.a.b &= 3`** / **`o.a.b |= 2`** / **`o.a.b ^= 3`** / **`xs[0].x &= 3`** / **`xs[0].x |= 2`** / **`xs[0].x ^= 3`** / **`o.items[0] &= 3`** / **`o.items[0] |= 2`** / **`o.items[0] ^= 3`** / **`o.items[0][0] &= 3`** / **`o.items[0][0] |= 2`** / **`o.items[0][0] ^= 3`** / **`xs[0][0].x &= 3`** / **`xs[0][0].x |= 2`** / **`xs[0][0].x ^= 3`** / **`xs[0][0] &= 3`** / **`xs[0][0] |= 2`** / **`xs[0][0] ^= 3`** / **`o.items[0][0].x &= 3`** / **`o.items[0][0].x |= 2`** / **`o.items[0][0].x ^= 3`** / **`xs[0][0][0] &= 3`** / **`xs[0][0][0] |= 2`** / **`xs[0][0][0] ^= 3`** / **`n <<= 1`** / **`n >>= 1`** / **`n >>>= 1`** / **`o.x <<= 1`** / **`o.x >>= 1`** / **`o.x >>>= 1`** / **`xs[0] <<= 1`** / **`xs[0] >>= 1`** / **`xs[0] >>>= 1`** / **`this.n <<= 1`** / **`this.n >>= 1`** / **`this.n >>>= 1`** / **`super.n <<= 1`** / **`super.n >>= 1`** / **`super.n >>>= 1`** / **`o.a.b <<= 1`** / **`o.a.b >>= 1`** / **`o.a.b >>>= 1`** / **`xs[0].x <<= 1`** / **`xs[0].x >>= 1`** / **`xs[0].x >>>= 1`** / **`o.items[0] <<= 1`** / **`o.items[0] >>= 1`** / **`o.items[0] >>>= 1`** / **`o.items[0][0] <<= 1`** / **`o.items[0][0] >>= 1`** / **`o.items[0][0] >>>= 1`** / **`xs[0][0].x <<= 1`** / **`xs[0][0].x >>= 1`** / **`xs[0][0].x >>>= 1`** / **`o.items[0][0].x <<= 1`** / **`o.items[0][0].x >>= 1`** / **`o.items[0][0].x >>>= 1`** / **`xs[0][0] <<= 1`** / **`xs[0][0] >>= 1`** / **`xs[0][0] >>>= 1`** / **`xs[0][0][0] <<= 1`** / **`xs[0][0][0] >>= 1`** / **`xs[0][0][0] >>>= 1`** / **`n **= 2`** / **`o.x **= 2`** / **`xs[0] **= 2`** / **`this.n **= 2`** / **`super.n **= 2`** / **`o.a.b **= 2`** / **`xs[0].x **= 2`** / **`o.items[0] **= 2`** / **`o.items[0][0] **= 2`** / **`xs[0][0].x **= 2`** / **`o.items[0][0].x **= 2`** / **`xs[0][0] **= 2`** / **`xs[0][0][0] **= 2`** / **`n %= 7`** / **`o.x %= 7`** / **`xs[0] %= 7`** / **`this.n %= 7`** / **`super.n %= 7`** / **`o.a.b %= 7`** / **`xs[0].x %= 7`** / **`o.items[0] %= 7`** / **`o.items[0][0] %= 7`** / **`xs[0][0].x %= 7`** / **`o.items[0][0].x %= 7`** / **`xs[0][0] %= 7`** / **`xs[0][0][0] %= 7`** / **`n -= 2`** / **`o.x -= 2`** / **`xs[0] -= 2`** / **`this.n -= 2`** / **`super.n -= 2`** / **`o.a.b -= 2`** / **`xs[0].x -= 2`** / **`o.items[0] -= 2`** / **`o.items[0][0] -= 2`** / **`xs[0][0].x -= 2`** / **`o.items[0][0].x -= 2`** / **`xs[0][0] -= 2`** / **`xs[0][0][0] -= 2`** / **`n *= 3`** / **`switch`** / **`fallthrough`** / **`do while`** / **`xs[0] +=`** / **`step()?`/`bad()?`** / **`? :`** / **`import.meta`** / **`` `n=${n}` ``** / **`||=` `&&=` `??=`** / **`??`** / **`for let i = 0`** / **`for x of`** / **`for k in`** / **`match 1 { 1 => 2, _ => 0 }`** / **`match [x, y]`** / **`match { p, q }`** / **`if let Some(x)`** / **`while let Ok(v)`** / **`match 1..=5`** / **`n @ 1..=5`** / **`1 | 2 | 3`** / **`..5`** / **`5..`** / **`[h, ...t]`** / **`{ k, ...s }`** / **`[h, ...mid, last]`** / **`n @ 1..=5 if n != 3`** / **`Color.Red`** / **`Msg.Move(p)`** / **`xs @ [p, q]`** / **`wrap @ { k, ...s }`** / **`{ k: n @ 1..=5 }`** / **`[n @ 1, ...r]`** / **`Ok(n @ 1..=5)`** / **`Some(n @ 1..=5)`** / **`if let 1 | 2`** / **`while let 1 | 2`** / **`(1 | 2)`** / **`Option.Some(n)`** / **`Option.Some("x")`** / **`Option<Number>.None`** / **`1.0..=2.0`** / **`Result.Ok(n)`** / **`Result<Number, String>.Err`** / **`if let n @ Some(x)`** / **`if let 1.. = x`** / **`while let 1.. = r`** / **`if let ..5 = x`** / **`while let ..5 = r`** / **`n @ 1 | 2`** / **`v @ Msg.Move(x)`** / **`struct Box<T>`** / **`Box$String`** / **`echo$Number`** / **`echo$String`** / **`Box<String>("hi")`** / **`Child$Number`** / **`id<Number>(42)`** / **`id$String`** / **`id(id(42))`** / **`pair$Number_String`** / **`id$Box`** / **`pair(x, s)`** / **`len(wrap(1))`** / **`super.init`** / **`super.count = 1`** / **`super.n += 2`** / **`let m = super.tag`** / **`this.run(super.f)`** / **`Show<Number>`** / **`type Item = Number`** / **`where T: Show`** / **`show_it<Shown>`** / **`Box().show_it<Shown>`** / **`show_it<Nope>`** / **`Box().show_it<Nope>`** / **`Box<Nope>`** / **`where T: Show, T: Named`** / **`both_it<OnlyShow>`** / **`where A: Show, B: Named`** / **`pair_it<Shown, Nope>`** / **`PairBox<Shown, Labeled>`** / **`PairBox<Shown, Nope>`** / **`Box().join_ab<Shown, Labeled>`** / **`Box().join_ab<Shown, Nope>`** / **`Box().both_it<Shown>`** / **`Box().both_it<OnlyShow>`** / **`BothBox<Shown>`** / **`BothBox<OnlyShow>`** / **`WBox<Shown>`** / **`WBox<Nope>`** / **`Thing().id()`** / **`id() { return 42 }`** / **`Show<T> default`** / **`Show<T> default override`** / **`is(obj, "Class")`** / **`pass`/`assert`/`not`** / **`raise`** / **`o.x +=`** / **`o.a.b +=`** / **`o.items[0] +=`** / **`o.items[0][0] +=`** / **`xs[0].x +=`** / **`xs[0][0].x +=`** / **`o.items[0][0].x +=`** / **`xs[0][0] +=`** / **`xs[0][0][0] +=`** / **`o.x ||= `** / **`o.x &&=`** / **`o.x ??=`** / **`xs[0] ||= `** / **`xs[0] ??=`** / **`o.a.b ??=`** / **`o.items[0] ||= `** / **`xs[0].x ??=`** / **`xs[0][0] ||= `** / **`this.n ||= `** / **`o.items[0][0] ||= `** / **`xs[0][0].x ??=`** / **`super.n ||= `** / **`o.items[0][0].x ??=`** / **`xs[0][0][0] ||= `** / **`Child<T> super.n ||= `**. 100 loop / 200 unrolled. 1k+ loop är nested-interpreter. Text-`maxKbc` oförändrad. Inte `noll_*`. Språkparitet i `.kab`, inte `src/`.

**Dok efter varje deepen:** uppdatera [docs/ROADMAP.md](../docs/ROADMAP.md) (status + **Nästa**) och den här filen (nuläge + milstolpar + tester) i samma pass. Språkparitet: även [docs/LANGUAGE.md](../docs/LANGUAGE.md).

## Kedja

```
source text
    → tokenizeExec / parseTokensExec   AST
    → emitMainExec                     opcode IR
    → serSerializeBc                   .kbc text
    → compile.kab                      source → .kbc
    → seed/compiler.kbcb               SH1 packed image
```

Default CLI: `kabootar compile` → self-host först. App-`.kab` har **ingen** Rust-fallback (SH16); `KABOOTAR_COMPILE=rust` / `--rust` **felar** för appar. `self_host/` DAG får rust-seeds. `bootPolicy("prefer")` = `self-host-only`.

## Nuläge (inte den gamla shard-listan)

| Yta | Status |
|-----|--------|
| Skip-list | **tom** (`attempt-all`, P6b) |
| Compile-DAG | **12** `.kab` (SH5 platå); `vm_*` **&lt; 40** (SH6) |
| Image | `self_host/seed/compiler.kbcb` + `seed/dag/*.kbc` (SH1) |
| Facader | `pub let` alias, inte wrapping `pub fn` (SH3b) |
| Lexer | **SH12:** `gLxSess` + in-place tokens; skip/ident/number cache `src`/`pos` |
| Parser/emit | **SH2/SH13:** återanvänd `gSess`/`gE` + `pResetSession`/`eResetSession`; tramp 0-arg. `pCondStack` på sess |
| **`struct` `&self`** | ✅ R4: `fn get(&self)` / `fn set(&mut self, n)` parse + compile-run (samma `self`-lokal som `fn sum(self)`) |
| **`match`** | ✅ produktkompilatorn: const/`_`/var/`Some`/`None`/`Ok`/`Err`/`NaN` + guards + **array/objekt** + **`...rest`** / **`[a, ...mid, b]`** + **or-mönster** (`1 \| 2`) + **range** (`1..5` / `1..=5` / **`..5` / `1..` / `..=5`**) + **`n @ pat`** + **`(pat)`** + **enum** (unit + payload-ctors) + **`if let`/`while let`**. Lexer: `..` / `..=` / `@`. Text-`.kbc` enum-sektion; host + Kab-VM |
| Dirty seeds | `compile_dirty_dag_seeds()` loggar `dirty=N` (SH7) |
| Produktträd | `compile_dirty_product_tree(entry)` (SH7b) |
| Tiny parse | `sh8_tiny_parse_via_compiler_image` i CI; full `compile("return 1")` ignored i debug |
| Cache | SH15 content-addressed `cache/ca/v{image}_{fp}.kbcb` + mmap |

Tunga `_*probe*` / `_bisect*` är **inte** produkt. Regenerera image: `KABOOTAR_SH1_WARM=1 cargo test --test sh_wave sh1_warm -- --ignored`.

## Filer (ingångar)

| Fil | Roll |
|-----|------|
| `compile.kab` | `compile(source)` / `compileIr` |
| `parse.kab` | `parse` = tokenizeExec + parseTokensExec |
| `lexer.kab` / `parser.kab` / `emit.kab` / `serialize.kab` | tunna `pub let`-facader |
| `parser_exec.kab` / `emit_exec.kab` | per-call session + tramp |
| `ownership.kab` | O5 `@manual` |
| `vm.kab` | kab-only VM (alias till `vm_run_exec_core`) |
| `deserialize.kab` / `deserialize_kbcb.kab` | text `.kbc` / packed kbcb v2 (Number array eller Uint8Array) → `runModule` IR |
| `seed/compiler.kbcb` | packed compile-DAG |

## Tester

```bash
cargo test --test sh_wave -- --test-threads=1
cargo test --test self_host self_host_parser_suite -- --test-threads=1
cargo test --test self_host self_host_if_let_some_compile_run -- --test-threads=1
cargo test --test self_host self_host_while_let_ok_compile_run -- --test-threads=1
cargo test --test self_host self_host_match_enum_pattern_compile_run -- --test-threads=1
cargo test --test self_host self_host_match_enum_payload -- --test-threads=1
cargo test --test self_host self_host_match_array_rest -- --test-threads=1
cargo test --test self_host self_host_match_object_rest -- --test-threads=1
cargo test --test self_host self_host_match_array_mid_rest -- --test-threads=1
cargo test --test self_host self_host_match_or_pattern -- --test-threads=1
cargo test --test self_host self_host_if_let_or -- --test-threads=1
cargo test --test self_host self_host_match_range -- --test-threads=1
cargo test --test self_host self_host_match_open_range -- --test-threads=1
cargo test --test self_host self_host_match_at_bind -- --test-threads=1
cargo test --test self_host self_host_if_let_at_bind -- --test-threads=1
cargo test --test self_host self_host_match_array_at_rest -- --test-threads=1
cargo test --test self_host self_host_match_ok_nested_at -- --test-threads=1
cargo test --test self_host self_host_while_let_at_ok -- --test-threads=1
cargo test --test self_host self_host_match_object_field_at -- --test-threads=1
cargo test --test self_host self_host_match_at_bind_array -- --test-threads=1
cargo test --test self_host self_host_match_at_bind_object -- --test-threads=1
cargo test --test self_host self_host_match_at_bind_guard -- --test-threads=1
cargo test --test self_host self_host_match_at_bind_object_rest -- --test-threads=1
cargo test --test self_host self_host_struct_ref_self -- --test-threads=1
cargo test --test self_host self_host_class_method_ok_kab_only -- --test-threads=1
cargo test --test self_host self_host_super_method_ok_kab_only -- --test-threads=1
cargo test --test self_host self_host_super_init_ok_kab_only -- --test-threads=1
cargo test --test self_host self_host_super_member_assign_ok_kab_only -- --test-threads=1
cargo test --test self_host self_host_super_bound_method_ok_kab_only -- --test-threads=1
cargo test --test self_host self_host_super_compound_assign_ok_kab_only -- --test-threads=1
cargo test --test self_host self_host_member_logical_assign_compile_run -- --test-threads=1
cargo test --test self_host self_host_mixed_logical_assign_compile_run -- --test-threads=1
cargo test --test self_host self_host_super_logical_assign_compile_run -- --test-threads=1
cargo test --test self_host self_host_super_callback_ok_kab_only -- --test-threads=1
cargo test --test self_host self_host_generic_super_method_ok_kab_only -- --test-threads=1
cargo test --test sh_wave sh6_default_eval -- --test-threads=1
cargo test --test sh_wave sh6_default_eval_in_ok -- --test-threads=1
cargo test --test sh_wave sh6_default_eval_await_ok -- --test-threads=1
cargo test --test sh_wave sh6_default_eval_array_slice_from_ok -- --test-threads=1
cargo test --test sh_wave sh6_default_eval_new_instance_from_array_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_let_array_rest_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_let_object_rest_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_array_literal_spread_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_object_literal_spread_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_is_is_not_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_let_nested_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_object_shorthand_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_object_method_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_object_computed_key_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_fn_default_param_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_fn_rest_param_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_arrow_default_rest_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_class_method_default_rest_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_object_method_default_rest_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_iface_default_method_default_rest_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_optional_chain_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_switch_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_do_while_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_switch_fallthrough_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_result_question_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_result_question_err_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_ternary_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_import_meta_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_template_literal_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_logical_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_classic_for_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_for_of_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_for_in_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_const_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_array_object_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_if_while_let_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_range_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_or_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_open_range_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_rest_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_mid_rest_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_guard_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_enum_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_at_nested_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_field_at_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_elem_at_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_payload_at_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_if_while_let_or_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_paren_or_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_option_enum_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_float_range_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_result_enum_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_if_let_at_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_at_or_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_payload_at_bind_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_match_option_two_specs_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_struct_box_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_struct_box_two_specs_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_method_echo_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_method_echo_two_specs_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_box_explicit_string_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_class_extends_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_fn_id_explicit_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_fn_id_two_specs_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_fn_id_nested_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_fn_pair_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_fn_id_box_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_fn_pair_from_lets_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_len_wrap_call_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_super_init_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_super_count_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_super_n_add_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_bound_tag_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_as_callback_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_trait_show_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_class_assoc_item_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_class_show_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_fn_show_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_method_show_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_fn_reject_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_method_reject_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_class_reject_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_fn_two_bounds_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_fn_two_bounds_reject_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_fn_pair_bounds_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_fn_pair_bounds_reject_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_class_pair_bounds_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_class_pair_bounds_reject_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_method_pair_bounds_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_method_pair_bounds_reject_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_method_two_bounds_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_method_two_bounds_reject_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_class_two_bounds_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_class_two_bounds_reject_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_struct_show_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_where_struct_show_reject_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_trait_default_id_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_trait_default_override_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_trait_default_generic_show_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_trait_default_generic_show_override_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_if_let_open_range_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_while_let_open_range_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_if_let_open_to_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_while_let_open_to_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_is_class_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_pass_assert_not_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_raise_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_member_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_mixed_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_member_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_member_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_logical_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_logical_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_member_nullish_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_mixed_logical_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_logical_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_logical_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_logical_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_member_logical_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_logical_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_member_logical_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_logical_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_super_logical_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_member_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_index_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_member_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_member_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_member_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_nested_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_computed_member_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_nested_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_member_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_computed_member_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_computed_member_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_member_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_nested_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_nested_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_index_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_nested_member_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_member_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_nested_member_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_member_index_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_member_index_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_member_nested_index_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_member_nested_index_computed_delete_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_bitwise_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_bitwise_or_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_bitwise_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_bitwise_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_bitwise_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_bitwise_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_bitwise_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_bitwise_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_bitwise_or_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_bitwise_and_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_bitwise_or_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_bitwise_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_bitwise_and_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_bitwise_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_member_bitwise_and_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_member_bitwise_or_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_member_bitwise_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_member_bitwise_and_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_member_bitwise_or_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_member_bitwise_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_index_bitwise_and_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_index_bitwise_or_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_index_bitwise_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_bitwise_and_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_bitwise_or_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_bitwise_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_member_bitwise_and_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_member_bitwise_or_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_member_bitwise_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_bitwise_and_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_bitwise_or_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_bitwise_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_member_bitwise_and_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_member_bitwise_or_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_member_bitwise_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_bitwise_and_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_bitwise_or_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_bitwise_xor_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_member_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_member_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_member_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_member_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_member_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_member_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_index_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_index_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_index_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_member_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_member_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_member_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_member_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_member_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_member_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_shl_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_shr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_ushr_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_member_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_member_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_index_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_member_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_member_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_pow_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_member_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_member_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_index_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_member_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_member_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_mod_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_this_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_super_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_member_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_member_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_index_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_member_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_member_nested_index_member_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_nested_index_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_sub_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_mul_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_triple_index_mul_compound_assign_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_index_compound_assign_eval_once_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_method_this_writeback_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_using_class_close_writeback_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generic_super_bound_tag_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_dynamic_import_math_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_async_fn_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_for_await_array_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_yield_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_array_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_for_of_generator_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_yield_star_generator_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_method_return_ok -- --test-threads=1
cargo test --test sh_wave sh6_self_host_generator_throw_catch_ok -- --test-threads=1
cargo test --test sh_wave sh6_kbcb_oversize_string_const_ok -- --test-threads=1
cargo test --test sh_wave sh6_kbcb_loop_100_ok -- --test-threads=1
cargo test --test sh_wave sh6_kbcb_unrolled_200_ok -- --test-threads=1
cargo test --test sh_wave sh6_kbcb_file_mmap_eval_ok -- --test-threads=1
cargo test --test sh_wave sh6_eval_file_cached_mmap_ok -- --test-threads=1
cargo test --test sh_wave sh6_kab_only_eval_ok -- --test-threads=1
cargo test --test sh_wave sh6_eval_file_cached_kbcb_image_ok -- --test-threads=1
cargo test --test self_host self_host_generic_struct_box -- --test-threads=1
cargo test --test self_host self_host_generic_method_on_specialized -- --test-threads=1
cargo test --test self_host self_host_generic_enum -- --test-threads=1
cargo test --test self_host self_host_trait_default_method -- --test-threads=1
cargo test --test self_host self_host_class_assoc_type -- --test-threads=1
cargo test --test self_host self_host_generic_trait -- --test-threads=1
cargo test --test self_host self_host_where_bound -- --test-threads=1
cargo test --test self_host self_host_where_method_bound -- --test-threads=1
cargo test --test self_host self_host_where_class_bound -- --test-threads=1
kabootar self_host/test_tiny.kab
kabootar compile self_host/sample.kab
```

## Designregler

**SH2:** parser/emit-cursors (`pPos`, `eOps`, …) ligger på **session-objektet**, inte nya modul-globaler. Trampolin: `sess["tramp"](sess)` så rekursion inte fångar en modul-`sess`. Nested `if`/`while` använder `pCondStack` / `eIfJmpStack` **på sess**. Nested named `fn` i en funktion: `emitNestedNamedFn` (save/restore `eFnOps`, `MakeArrow` + lokal).

1. **Fn-lokaler** — bytecode speglar lokaler på *aktuell* aktiveringsram. Captures: `local_captures`. **Lexer-ident:** `let cd`/`ok`/`start` i samma fn som loopen (`lxScanIdent`) — saknad `let` blir bytecode-global (`Undefined variable: cd`).
2. **`push` returnerar ny array** — skriv `arr = push(arr, item)`.
3. **Spara AST-fält före rekursion** — t.ex. `eSym = eNode["sym"]` innan `emitExpr(init)`.
4. **Bracket-access för AST-nycklar** — `node["sym"]` där `.then`/`.value` krockar.
5. **Radbrytning** — använd `CHAR_NL`, inte `"\n"` i serializer (SH3c).
6. **Assign: peek före bump** — `let tok = peek(); bump();`.
7. **Modulskala (L2)** — ≥40 top-level `fn` per modul OK. Densify till 5-radersfiler är **föråldrat** (ökar import-evals).
8. **Exporterade fn** — `pub let X = Ximpl` på facader (SH3b); wrapping `pub fn` ger extra Kab-VM-ram.
9. **Nested import** — `import "self_host/compile"` för hela kedjan; `parse.kab` för AST-only. Importera inte `parser.kab` tillsammans med `parse.kab`.
10. **CALL-args** — undvik var+literal i samma 2-arg `CALL` i heta fn.
11. **Windows stack** — `build.rs` sätter 16 MiB för `kabootar`-bin.
12–51. Nested if/while/call/clobber-workarounds (`pCondStack`, `eIfJmpStack`, `eCalleeStack`, …) är **session-fält**, inte nya modul-globaler. Full lista historiskt nedan; nya `let pSave*` / `let pPos` i facader är **förbjudna** (SH10).

## Historiska clobber-regler (session-fält, inte nya globals)

12. **Compare-parse** — spara lhs före rhs; inte `parsePostfix()` för compare-rhs.
13. **Emitter while** — loop-head i `eWhileHead`; jump-args relativa: `target - jmpIndex - 1`.
14. **Bytecode-cache** — `.kabootar/cache/*.kbc` + fingerprint (content + import-mtimes). SH7b kompilerar bara dirty.
15. **Serialize från `.kbc`** — `CHAR_NL`; inte privata fn från exporterade `serialize_bc`.
16. **Array literal** — `AST_ARRAY` + `make_array`.
17. **Emitter scratch** — `eBxL`/`eBxR` / `eList` / `eBodyStmts` på sess.
18. **Throw** — `AST_THROW` + `throw` opcode.
19. **Nested if/while** — `eIfJmpStack`/`eIfSkipStack`; trimma `eBreakIdxs` efter inner loop.
20. **Parser sym snapshot** — `pFnSym`/`pFnPub` på sess före rekursiv `parseStmt`.
21. **while/if cond** — `pCondStack` på sess.
22. **let/assign sym** — `pBindSym` (inte `pSaveSym`).
23. **assign lookahead** — `pNextTok = pToks[pPos+1]` med EOF-fallback.
24. **bracket index** — fn-lokal `indexObj`.
25. **compare rhs** — `pInAddSub`.
26. **`+`/`-` rhs** — `pAddLeftStack` + rekursiv `parseCompare`.
27. **&& expr** — `pExprLeft`.
28. **binary op** — fn-lokaler `binOp`/`binRight`.
29. **pub exports** — `isPub` / `eExports` / `exports=`.
30. **let/member** — `eStoreSym` / `eMemberFldStack` / `eAssignSym` / `eExprStmt`.
31. **module globals in fn** — `eFnLocals` först, sedan `eGlobals`.
32. **fn snapshot** — `snapArr(eFnOps)` vid push till `eFunctions`.
33. **block loop** — `eBlockIStack`/`eBlockNStack`.
34. **expr-loops** — `eObjIStack`, `eArrIStack`, `eCallArgIStack`.
35. **parseTokens EOF** — `while pDone == 0`.
36. **binary `+` i fn** — spara rhs före `emitExpr(left)`.
37. **let sym** — `pLetSym` före `bump()`.
38. **undefined literal** — `TOKEN_UNDEFINED` → `LIT_UNDEF`.
39. **postfix chains** — interleaved `()`, `.`, `[]`.
40. **`null` vs `undefined`** — `null == undefined` är `false`.
41. **Program body** — block-stack + `OP_HALT`.
42. **index assign** — `AST_INDEX_ASSIGN` + `OP_INDEX_SET`.
43. **emit index assign** — spara `eBxRhs` före `emitExpr`.
44. **popStack** — native `pop(stack)`.
45. **import emit vs compile(emit.kab)** — häng via `.kbc` → serialize/compile, inte bara emit-logik.
46. **SH3a** — nested call args on locals (`callArgs`), not `sess["pArgs"]`; `len(expr)` → `get_length`; argv N-path. Gate: `self_host_len_of_call_expr_*` / `self_host_emit_nested_call_argn_*`.
47. **CHAR_NL** i serialize (SH3c).
48. **nested call emit** — `eCalleeStack`.
49. **nested call parse** — fn-lokaler `savedCallee`/`savedTypeArgs`.
50. **generic call type args** — `savedTypeArgs` med call.
51. **generic emit** — `eGenericTemplates`; ingen extra import från `emit.kab`.

## Nästa milstolpar (Våg SH)

Historiska 1–14 (roundtrip, facader, bootstrap, generics) är klara. **Inte nästa:** fler `_probe`-filer.

**~~SH17–SH27~~ ✅ subset** nedan är **flag-gates i `.kab`**, inte att Rust är raderad. Produktmålet är [noll Rust / SH28](../docs/ROADMAP.md#kabootar-på-egna-fötter--noll-rust): `src/` finns, rustc krävs, **SH28 är inte stängd**.

Kort ordning:

1. ~~SH0/SH1~~ ✅ · **SH2** nested named `fn` + sess ✅
2. ~~SH3–SH7b~~ ✅ · ~~SH5 densify~~ ✅ (serialize_sections+out+ir_line+acc, parser_expr→exec, parser_hooks/lexer_defs/emit_defs→ast_defs, lexer_tokenize→scan, emit_fn_scope/hooks/arr_util→sym, emit_sym_index→sym, emit_tramp/main_fn→exec, parser_main/tramp/type_args/session→exec, parser_block→hooks)
3. ~~**SH16**~~ ✅ appar: ingen rust-emit (`eval_file_cached` / `compile --rust`); toolchain `self_host/` får rust
4. **SH5 platå** — compile-DAG **12**; `ownership` får **inte** `pub import compile` (suiten laddar hela pipelinen). Inte `parser_stmt`/`postfix`/`emit_*_body` förrän leaf ≤10 s / ~550 rader. **`match`**: enum unit + payload + `if let`/`while let`.
5. ~~**SH17/SH18**~~ ✅ subset + deepen (`jitMmapOk` mmap/exec dual-bind; loop8/loopN/arith-imm/bit-ops/shifts/unary/eq/ne/lt/gt/le/ge `os_mm_call`; `gcHostDeleteOk` host-GC dual-bind)
6. ~~**SH19**~~ ✅ subset + deepen (`loadMainDeleteOk` main.rs dual-bind)
7. ~~**SH20**~~ ✅ subset (JSON/datum/regex + math + objekt + collections + colget + collen + colpush + colpop + colfirst + collast + colrest + colempty + colconcat + colrev + colcontains + colindex + colcount + coltake + coldrop + colzip + colunzip + colflat + colunique + coleq + colclone + colrepeat + colfill + colrange + colsum + colmax + colmin + colprod + colavg + colmed + colmode + colsort + coldesc + colfind + colrfind + colrix + colslice + colwin + colchunk + colrot + colpad + colilv + coltr + coldiag + colident + coltrc + colrow + colcol + colshape + colrshp + coldot + colmv + colmm + colout + colcrs + coldet + colnorm + colunit + colproj + colrej + coldist + collerp + colscale + coladd + colsub + colmul + coldiv + colneg + colabs + colsign + colclamp + colmod + colpow + colsqrt + colsqr + colcub + colfloor + colceil + colround + coltrunc + collog + collog2 + collog10 + colexp + colsin + colcos + coltan + colasin + colacos + colatan + colatan2 + colhypot + colcbrt + colimul + colclz32 + colfround + colf16round + colsumprec + collog1p + colexpm1 + colsinh + colcosh + coltanh + colasinh + colacosh + colatanh + colfmod + colrandom + colpi + cole + colln2 + colln10 + collog2e + collog10e + colsqrt2 + colsqrt12 leaves); radera natives deepen
8. ~~**SH21**~~ ✅ subset (`kabOsIsFile` + `kabOsArgvOk` + `kabOsEnvOk` + `kabOsCwdOk` + `kabOsIsDir` + `kabOsJoin` + `kabOsBase` + `kabOsExt` + `kabOsDirname` + `kabOsNorm` + `kabOsAbs` + `kabOsRel`); radera `runtime/os` deepen
9. ~~**SH22**~~ ✅ subset (`sqlIsWhere` + `sqlStoreOk` + `sqlIsLimit` + `sqlIsOrder` + `sqlIsInsert` + `sqlIsUpdate` + `sqlIsDelete` + `sqlIsCreate` + `sqlIsJoin` + `sqlIsGroup` + `sqlIsHaving` + `sqlIsDistinct` + `sqlIsUnion`); radera `src/sql` deepen
10. ~~**SH23**~~ ✅ subset (`cryptoTls12Ok` + `cryptoRootPem` + `cryptoTls13Ok` + `cryptoSha256Ok` + `cryptoHmacOk` + `cryptoAes256Ok` + `cryptoChaChaOk` + `cryptoGcmOk` + `cryptoEd25519Ok` + `cryptoX25519Ok`); rustls-delete deepen
11. ~~**SH24**~~ ✅ subset (`httpIsPost` + `httpIsJson` + `httpIsPut` + `httpIsPatch` + `httpIsHead` + `httpIsDelete` + `httpIsOptions` + `httpIsTrace` + `httpIsConnect`); radera `runtime/http.rs` deepen
12. ~~**SH25**~~ ✅ subset (`cliIsCompile` + `cliIsFmt` + `cliIsCheck` + `cliIsLint` + `cliIsVersion` + `cliIsHelp` + `cliIsDoc` + `cliIsBench` + `cliIsNew` + `cliIsInit` + `cliIsWatch` + `cliIsClean` + `cliIsAdd` + `cliIsRm` + `cliIsMod` + `cliIsLs` + `cliIsCat`); radera `src/cli` deepen
13. ~~**SH26**~~ ✅ subset (`sciNdLenOk` + `sciFftPow2` + `sciSub` + `sciDiv` + `sciNeg` + `sciAbs` + `sciMax` + `sciMin` + `sciClamp` + `sciPow` + `sciSqr` + `sciCub` + `sciSign`); GPU kernel deepen
14. ~~**SH27**~~ ✅ subset (`uiIsCanvas` + `uiFpsOk` + `uiIsSpan` + `uiIsButton` + `uiIsInput` + `uiIsImg` + `uiIsP` + `uiIsA` + `uiIsUl` + `uiIsLi` + `uiIsOl` + `uiIsH1` + `uiIsH2` + `uiIsH3` + `uiIsH4` + `uiIsH5` + `uiIsH6` + `uiIsForm` + `uiIsLabel` + `uiIsTextarea` + `uiIsSelect` + `uiIsOption` + `uiIsTable` + `uiIsTr` + `uiIsTh` + `uiIsTd` + `uiIsThead` + `uiIsTbody` + `uiIsTfoot` + `uiIsNav` + `uiIsHeader` + `uiIsFooter` + `uiIsMain` + `uiIsSection` + `uiIsArticle` + `uiIsAside` + `uiIsFigure` + `uiIsFigcaption` + `uiIsDetails` + `uiIsSummary` + `uiIsDialog` + `uiIsPre` + `uiIsCode` + `uiIsBlockquote` + `uiIsVideo` + `uiIsAudio` + `uiIsSource` + `uiIsTrack` + `uiIsIframe` + `uiIsFieldset` + `uiIsLegend` + `uiIsHr` + `uiIsBr` + `uiIsKbd` + `uiIsSamp` + `uiIsVar` + `uiIsAbbr` + `uiIsCite` + `uiIsMark` + `uiIsSmall` + `uiIsStrong` + `uiIsEm` + `uiIsSub` + `uiIsSup` + `uiIsTime` + `uiIsQ` + `uiIsB` + `uiIsI` + `uiIsU` + `uiIsS` + `uiIsDel` + `uiIsIns` + `uiIsWbr` + `uiIsRuby` + `uiIsRt` + `uiIsRp` + `uiIsBdi` + `uiIsBdo` + `uiIsData` + `uiIsDfn` + `uiIsMeter` + `uiIsProgress` + `uiIsOutput` + `uiIsDatalist` + `uiIsOptgroup` + `uiIsPicture` + `uiIsMap` + `uiIsArea` + `uiIsEmbed` + `uiIsObject` + `uiIsParam` + `uiIsColgroup` + `uiIsCol` + `uiIsCaption` + `uiIsTemplate` + `uiIsSlot` + `uiIsNoscript` + `uiIsScript` + `uiIsStyle` + `uiIsLink` + `uiIsMeta` + `uiIsTitle` + `uiIsBase` + `uiIsHead` + `uiIsBody` + `uiIsHtml` + `uiIsHgroup` + `uiIsAddress` + `uiIsDl` + `uiIsDt` + `uiIsDd` + `uiIsMenu` + `uiIsSearch` + `uiIsPortal` + `uiIsSvg` + `uiIsMath` + `uiIsSelectedcontent` + `uiIsFencedframe` + `uiIsFrameset` + `uiIsFrame` + `uiIsNoframes` + `uiIsMarquee` + `uiIsFont` + `uiIsCenter` + `uiIsNobr` + `uiIsDir` + `uiIsBlink` + `uiIsApplet` + `uiIsBasefont` + `uiIsIsindex` + `uiIsKeygen` + `uiIsListing` + `uiIsXmp` + `uiIsPlaintext` + `uiIsMenuitem` + `uiIsNoembed` + `uiIsSpacer` + `uiIsBgsound` + `uiIsAcronym` + `uiIsBig` + `uiIsTt` + `uiIsStrike` + `uiIsRb` + `uiIsRtc` + `uiIsRbc` + `uiIsShadow` + `uiIsContent` + `uiIsElement` + `uiIsNextid` + `uiIsLayer` + `uiIsIlayer` + `uiIsNolayer` + `uiIsMulticol` + `uiIsComment` + `uiIsXml` + `uiIsImage` + `uiIsServer` + `uiIsDiv` + `uiIsRect` + `uiIsCircle` + `uiIsEllipse` + `uiIsLine` + `uiIsPolyline` + `uiIsPolygon` + `uiIsPath` + `uiIsG` + `uiIsUse` + `uiIsDefs` + `uiIsSymbol` + `uiIsMarker` + `uiIsClipPath` + `uiIsMask` + `uiIsPattern` + `uiIsLinearGradient` + `uiIsRadialGradient` + `uiIsStop` + `uiIsText` + `uiIsTspan` + `uiIsTextPath` + `uiIsForeignObject` + `uiIsSwitch` + `uiIsFilter` + `uiIsFeGaussianBlur` + `uiIsFeBlend` + `uiIsFeColorMatrix` + `uiIsFeComponentTransfer` + `uiIsFeComposite` + `uiIsFeConvolveMatrix` + `uiIsFeDiffuseLighting` + `uiIsFeDisplacementMap` + `uiIsFeFlood` + `uiIsFeFuncA` + `uiIsFeFuncB` + `uiIsFeFuncG` + `uiIsFeFuncR` + `uiIsFeImage` + `uiIsFeMerge` + `uiIsFeMergeNode` + `uiIsFeMorphology` + `uiIsFeOffset` + `uiIsFePointLight` + `uiIsFeSpecularLighting` + `uiIsFeSpotLight` + `uiIsFeTile` + `uiIsFeTurbulence` + `uiIsFeDistantLight` + `uiIsFeDropShadow` + `uiIsAnimate` + `uiIsAnimateMotion` + `uiIsAnimateTransform` + `uiIsSet` + `uiIsMpath` + `uiIsView` + `uiIsMetadata` + `uiIsDesc` + `uiIsHatch` + `uiIsHatchpath` + `uiIsSolidcolor` + `uiIsCursor` + `uiIsTref` + `uiIsAltGlyph` + `uiIsAltGlyphDef` + `uiIsAltGlyphItem` + `uiIsGlyphRef` + `uiIsGlyph` + `uiIsMissingGlyph` + `uiIsFontFace` + `uiIsFontFaceSrc` + `uiIsFontFaceUri` + `uiIsFontFaceFormat` + `uiIsFontFaceName` + `uiIsHkern` + `uiIsVkern` + `uiIsMeshgradient` + `uiIsMeshrow` + `uiIsMeshpatch` + `uiIsDiscard` + `uiIsUnknown` + `uiIsMrow` + `uiIsMi` + `uiIsMn` + `uiIsMo` + `uiIsMtext` + `uiIsMs` + `uiIsMspace` + `uiIsMfrac` + `uiIsMsqrt` + `uiIsMroot` + `uiIsMsub` + `uiIsMsup` + `uiIsMsubsup` + `uiIsMunder` + `uiIsMover` + `uiIsMunderover` + `uiIsMmultiscripts` + `uiIsMprescripts` + `uiIsNone` + `uiIsMtable` + `uiIsMtr` + `uiIsMtd` + `uiIsMth` + `uiIsMlabeledtr` + `uiIsMaligngroup` + `uiIsMalignmark` + `uiIsMstyle` + `uiIsMerror` + `uiIsMpadded` + `uiIsMphantom` + `uiIsMfenced` + `uiIsMenclose` + `uiIsSemantics` + `uiIsAnnotation` + `uiIsAnnotationXml` + `uiIsMaction` + `uiIsMlongdiv` + `uiIsMstack` + `uiIsMsrow` + `uiIsMscarries` + `uiIsMscarry` + `uiIsMsline` + `uiIsMglyph` + `uiIsCi` + `uiIsCn` + `uiIsCsymbol` + `uiIsApply` + `uiIsBind` + `uiIsBvar` + `uiIsShare` + `uiIsCondition` + `uiIsPiecewise` + `uiIsPiece` + `uiIsOtherwise` + `uiIsLambda` + `uiIsReln` + `uiIsFn` + `uiIsInterval` + `uiIsList` + `uiIsVector` + `uiIsMatrix` + `uiIsMatrixrow` + `uiIsSelector` + `uiIsDomain` + `uiIsCodomain` + `uiIsDomainof` + `uiIsIdent` + `uiIsCompose` + `uiIsInverse` + `uiIsPlus` + `uiIsMinus` + `uiIsTimes` + `uiIsDivide` + `uiIsPower` + `uiIsRoot` + `uiIsGcd` + `uiIsAnd` + `uiIsOr` + `uiIsXor` + `uiIsNot` + `uiIsImplies` + `uiIsForall` + `uiIsExists` + `uiIsEquivalent` + `uiIsApprox` + `uiIsFactorof` + `uiIsTendsto` + `uiIsInt` + `uiIsDiff` + `uiIsPartialdiff` + `uiIsLowlimit` + `uiIsUplimit` + `uiIsDegree` + `uiIsLogbase` + `uiIsLog` + `uiIsLn` + `uiIsExp` + `uiIsSin` + `uiIsCos` + `uiIsTan` + `uiIsSec` + `uiIsCsc` + `uiIsCot` + `uiIsSinh` + `uiIsCosh` + `uiIsTanh` + `uiIsSech` + `uiIsCsch` + `uiIsCoth` + `uiIsArcsin` + `uiIsArccos` + `uiIsArctan` + `uiIsArccosh` + `uiIsArccot` + `uiIsArccoth` + `uiIsArccsc` + `uiIsArccsch` + `uiIsArcsec` + `uiIsArcsech` + `uiIsArcsinh` + `uiIsArctanh` + `uiIsAbs` + `uiIsConjugate` + `uiIsArg` + `uiIsReal` + `uiIsImaginary` + `uiIsFloor` + `uiIsCeiling` + `uiIsMin` + `uiIsMax` + `uiIsLcm` + `uiIsMean` + `uiIsSdev` + `uiIsVariance` + `uiIsMedian` + `uiIsMode` + `uiIsMoment` + `uiIsMomentabout` + `uiIsCartesianproduct` + `uiIsVectorproduct` + `uiIsScalarproduct` + `uiIsOuterproduct` + `uiIsTranspose` + `uiIsDeterminant` + `uiIsUnion` + `uiIsIntersect` + `uiIsIn` + `uiIsNotin` + `uiIsSubset` + `uiIsPrsubset` + `uiIsNotsubset` + `uiIsNotprsubset` + `uiIsSetdiff` + `uiIsCard` + `uiIsSum` + `uiIsProduct` + `uiIsLimit` + `uiIsCurl` + `uiIsDivergence` + `uiIsGrad` + `uiIsLaplacian` + `uiIsEmptyset` + `uiIsIntegers` + `uiIsRationals` + `uiIsReals` + `uiIsComplexes` + `uiIsNaturalnumbers` + `uiIsPrimes` + `uiIsExponentiale` + `uiIsImaginaryi` + `uiIsPi` + `uiIsEulergamma` + `uiIsInfinity` + `uiIsNotanumber` + `uiIsTrue` + `uiIsFalse` + `uiIsDomainofapplication` + `uiIsSep` + `uiIsDeclare` + `uiIsCerror` + `uiIsCs` + `uiIsCbytes` + `uiIsMsgroup` + `uiIsNeq` + `uiIsLt` + `uiIsGt` + `uiIsLeq` + `uiIsGeq` + `uiIsRem` + `uiIsQuotient` + `uiIsFactorial` + `uiIsEq` + `uiIsAnimateColor` + `uiIsColorProfile` + `uiIsDefinitionSrc` + `uiIsPrefetch` + `uiIsHandler` + `uiIsListener` + `uiIsAnimation` + `uiIsTbreak` + `uiIsTextArea` + `uiIsFlowRoot` + `uiIsFlowRegion` + `uiIsFlowDiv` + `uiIsFlowPara` + `uiIsFlowSpan` + `uiIsFlowLine` + `uiIsFlowTref` + `uiIsFlowRegionExclude` + `uiIsFlowImage` + `uiIsSolidColor` + `uiIsMesh`); kbrowser deepen
15. **SH28 inte klar** — `src/` är produkt-skuld (`nollDropSrc=false`, `nollUserNoRustc=false`); flag-log (`nollSrcGoalZero=0` + `nollBootstrapFromKabOk=true` + `nollAllGatesClosedOk=true` + `nollAotReady=false` + `nollAotProcess=false` + `nollImageIsProcess=false` + `nollMmapExecProcess=false` + `nollStubIsProcess=false` + `nollSyscallIsKab=false` + `nollRustcNotHost=false` + `nollHostOptional=false` + `nollNoNewRs=true` + `nollKeepSrc` + `nollDropSrc=false` + `nollProcessIsKab=false` + `nollBootstrapImage` + `nollCargoNotRuntime=false` + `nollRustcNotProcess=false` + `nollMmapStub=false` + `nollStubFrozen=false` + `nollHostSyscallGone=false` + `nollProductSrcGone=false` + `nollCargoTomlGone=false` + `nollRustcCiGone=false` + `nollKabtestProductCi=false` + `nollUserNoRustc=false` + `nollCraneliftGone=false` + `nollJitIsKab=false` + `nollVmIsKab=false` + `nollGcIsKab=false` + `nollCompileIsKab=false` + `nollParseIsKab=false` + `nollLexIsKab=false` + `nollTypeIsKab=false` + `nollEmitIsKab=false` + `nollOptIsKab=false` + `nollLinkIsKab=false` + `nollStdIsKab=false` + `nollCliIsKab=false` + `nollReplIsKab=false` + `nollFmtIsKab=false` + `nollLspIsKab=false` + `nollDocIsKab=false` + `nollBenchIsKab=false` + `nollNewIsKab=false` + `nollInitIsKab=false` + `nollWatchIsKab=false` + `nollCleanIsKab=false` + `nollAddIsKab=false` + `nollRmIsKab=false` + `nollModIsKab=false` + `nollLsIsKab=false` + `nollCatIsKab=false` + `nollPkgIsKab=false` + `nollLockIsKab=false` + `nollPubIsKab=false` + `nollRegIsKab=false` + `nollAuthIsKab=false` + `nollLogIsKab=false` + `nollDbgIsKab=false` + `nollProfIsKab=false` + `nollTraceIsKab=false` + `nollCovIsKab=false` + `nollFuzzIsKab=false` + `nollSanIsKab=false` + `nollSnapIsKab=false` + `nollMockIsKab=false` + `nollFixIsKab=false` + `nollSpyIsKab=false` + `nollFakeIsKab=false` + `nollClockIsKab=false` + `nollRandIsKab=false` + `nollNetIsKab=false` + `nollDnsIsKab=false` + `nollTlsIsKab=false` + `nollHttpIsKab=false` + `nollWsIsKab=false` + `nollUdpIsKab=false` + `nollQuicIsKab=false` + `nollIcmpIsKab=false` + `nollSctpIsKab=false` + `nollGrpcIsKab=false` + `nollMqttIsKab=false` + `nollSmtpIsKab=false` + `nollImapIsKab=false` + `nollPopIsKab=false` + `nollFtpIsKab=false` + `nollSshIsKab=false` + `nollLdapIsKab=false` + `nollNtpIsKab=false` + `nollSnmpIsKab=false` + `nollDhcpIsKab=false` + `nollTftpIsKab=false` + `nollRadiusIsKab=false` + `nollKerberosIsKab=false` + `nollOauthIsKab=false` + `nollOidcIsKab=false` + `nollSamlIsKab=false` + `nollJwtIsKab=false` + `nollJwksIsKab=false` + `nollWebauthnIsKab=false` + `nollTotpIsKab=false` + `nollHotpIsKab=false` + `nollArgonIsKab=false` + `nollScryptIsKab=false` + `nollBcryptIsKab=false` + `nollPbkdfIsKab=false` + `nollHkdfIsKab=false` + `nollHmacIsKab=false` + `nollShaIsKab=false` + `nollAesIsKab=false` + `nollChachaIsKab=false` + `nollPolyIsKab=false` + `nollX25519IsKab=false` + `nollEd25519IsKab=false` + `nollKyberIsKab=false` + `nollDilithiumIsKab=false` + `nollSphincsIsKab=false` + `nollFalconIsKab=false` + `nollNtruIsKab=false` + `nollMcelieceIsKab=false` + `nollBikeIsKab=false` + `nollHqcIsKab=false` + `nollFrodoIsKab=false` + `nollSikeIsKab=false` + `nollRainbowIsKab=false` + `nollGemssIsKab=false` + `nollPicnicIsKab=false` + `nollXmssIsKab=false` + `nollLmsIsKab=false` + `nollWotsIsKab=false` + `nollMerkleIsKab=false` + `nollSlhdsaIsKab=false` + `nollMlkemIsKab=false` + `nollMldsaIsKab=false` + `nollFndsaIsKab=false` + `nollHybridIsKab=false` + `nollXwingIsKab=false` + `nollHpkeIsKab=false` + `nollNoiseIsKab=false` + `nollAgeIsKab=false` + `nollPgpIsKab=false` + `nollMinisignIsKab=false` + `nollSignifyIsKab=false` + `nollCosignIsKab=false` + `nollNotaryIsKab=false` + `nollRekorIsKab=false` + `nollFulcioIsKab=false` + `nollSigstoreIsKab=false` + `nollIntotoIsKab=false` + `nollSlsaIsKab=false` + `nollSpdxIsKab=false` + `nollCyclonedxIsKab=false` + `nollSbomIsKab=false` + `nollVexIsKab=false` + `nollCveIsKab=false` + `nollCweIsKab=false` + `nollCpeIsKab=false` + `nollCvssIsKab=false` + `nollOsvIsKab=false` + `nollGhsaIsKab=false` + `nollNvdIsKab=false` + `nollKevIsKab=false` + `nollCisaIsKab=false` + `nollMitreIsKab=false` + `nollAttackIsKab=false` + `nollCapecIsKab=false + nollStixIsKab=false + nollTaxiiIsKab=false + nollMispIsKab=false + nollOpenctiIsKab=false + nollThehiveIsKab=false + nollCortexIsKab=false + nollShuffleIsKab=false + nollWazuhIsKab=false + nollOsqueryIsKab=false + nollSuricataIsKab=false + nollZeekIsKab=false + nollSnortIsKab=false + nollYaraIsKab=false + nollSigmaIsKab=false + nollClamavIsKab=false + nollVirustotalIsKab=false + nollHybridanalysisIsKab=false + nollAnyrunIsKab=false + nollCuckooIsKab=false + nollJoeIsKab=false + nollCapeIsKab=false`); **radera inte `src/`**
16. ~~**F10 AOT native-image policy**~~ ✅ (ret-stub + sym/reloc + `nollAotReady` dual-bind); ~~**SH17–SH19 deepen**~~ ✅ (`jitMmapOk`, loopN arith-imm + bit-ops/shifts/unary/eq/ne/lt/gt/le/ge exec, `gcHostDeleteOk`, `loadMainDeleteOk` still false)
17. ~~**F18 App-CI**~~ ✅ subset (`appCiHttpOk` ≤100 ms + `appCiFpsOk` ≥60 + `appCiNdOk` ≤50 ms); live-app deepen — harness i `.kab`, inte ny `src/`-profiler
18. ~~**F19 zero-copy**~~ ✅ subset (`zcSqlViewOk` + `zcNdViewOk` 0 extra copies); live buffer/histogram deepen
19. ~~**F23 mmap-bulk**~~ ✅ subset (`mmF64Ok` f64 stride 8 + `mmU8Ok` u8 stride 1); GPU-pekare deepen
20. ~~**F22 TLAB**~~ ✅ subset (`tlabOk` ≥2 workers + `tlabPromoteOk` full→old dual-bind `gcHostDeleteOk`); pause-histogram deepen
21. ~~**F20 typspec**~~ ✅ subset (`jitSpecOk` i64 + `jitSpecF64Ok` f64 + `jitSpecDeoptOk` ny typ dual-bind `icIsMono`); native clone deepen
22. ~~**F24 TCO**~~ ✅ subset (`jitTcoOk` `@tail` + `jitTcoSelfOk` self-recursion dual-bind `jitSsaOk`); frame-reuse deepen
23. ~~**F21 lazy AOT**~~ ✅ subset (`aotLazyOk` ≤100 ms + `aotLazyJitOk` remainder JIT dual-bind `aotPgoOk`); live boot-DAG deepen

## Historisk bootstrap-logg

1. ~~`.kbc` roundtrip: `deserialize(serialize_bc(emit(ast)))` i Rust~~ ✅
2. ~~`fn`-anrop: `OP_CALL` mot self-hosted `functions[]`~~ ✅ (Rust `run_module`)
3. ~~`parse.kab`-facaden (nested `tokenize`)~~ ✅
4. ~~Full pipeline: `compile(source)` entrypoint~~ ✅
5. ~~Self-host bootstrap: `compile.kab` cache + `compile(sample)` -> Rust `run_module`~~ ✅
6. ~~Utöka self-hosted språksubset (obj, &&, compares, index)~~ ✅
7. ~~Lexer-like compile (`char_at`-loop, `!=`, `continue`/`break`/`undefined`)~~ ✅
8. ~~Self-host hela `lexer.kab` via `compile()`~~ ✅ — `self_host_lexer_full_compile_and_run` (~2.5 h)

9. ~~Self-host `parser.kab` / `emit.kab` (större moduler, fler opcodes).~~ ✅
   - **parser.kab** (~960 rader, 9 fn): generics (`<T>` på fn/class/enum, type args på call/member), `self_host_parser_suite` via Rust bytecode-preload (undviker Windows OOM). `self_host_parser_full_compile_and_run` (~2.5 h) verifierar `compile(parser.kab)` → `parseTokens(tokenize("let x = 1"))`.
   - **emit.kab** (~850 rader, 8 fn): redo för vidare opcode-stöd om parsern utökas (`||`, unary `!`, `*`, assign till index, etc.).
    - **Verifiering:** snabb: `self_host_emit_suite` (3 subprocess-chunks: core / generics / calls — undviker Windows OOM), `self_host_parser_full_compile_smoke`, `self_host_emit_full_compile_smoke`; långsam: `self_host_parser_full_compile_and_run` (ignored, ~2.5 h).

10. ~~Self-host hela `emit.kab` via `compile()` → kör `emit(parse("let x = 1"))` i bytecode~~ ✅
    - Snabb smoke: `self_host_emit_full_compile_smoke`.
    - Långsam CI: `self_host_emit_full_compile_and_run` (ignored, ~2–3 h).
    - Run-only: `self_host_emit_kbc_run_only` (kräver `_emit_full_out.kbc`).

11. ~~Self-host hela `serialize.kab` via `compile()` → kör `serialize_bc(emit(parse(...)))` + roundtrip~~ ✅
    - Snabb smoke: `self_host_serialize_full_compile_smoke`.
    - Långsam CI: `self_host_serialize_full_compile_and_run` (ignored, ~40 min).
    - Run-only: `self_host_serialize_kbc_run_only` (kräver `_serialize_full_out.kbc`).
    - Bygg KBC: `python scripts/profile_emit_compile.py compile serialize.kab`
    - **Kör tunga tester med `--test-threads=1`** (parallella serialize-tester kan OOM:a på Windows).

12. ~~True bootstrap — `compile(compile.kab)` körs som self-hosted bytecode och kan `compile(sample)`~~ ✅
    - Snabb smoke: `self_host_compile_full_compile_smoke` (subprocess — djup pipeline overflowar test-stack).
    - Långsam CI: `self_host_compile_full_compile_and_run` (ignored, ~3 min för compile.kab).
    - Run-only: `self_host_compile_kbc_run_only` (kräver `_compile_full_out.kbc`).
    - Bygg KBC: `python scripts/profile_emit_compile.py compile compile.kab`

13. ~~**Generics (språk):** Rust v1 + self-host G4~~ ✅ — `fn id<T>`, monomorphisering, `tests/generics.rs`, `test_parser.kab` / `test_emit.kab`. Design: [docs/GENERICS.md](../docs/GENERICS.md). **Struct** ✅ — `self` / `&self` / `&mut self` (self-host parse+emit); **`struct Box<T>`** med fälttyp `T` → `Number`; **G8.1** `b.echo(1)` → `echo$Number`; **`class Child<T> extends Base<T>`** → `Child$Number` extends `Base$Number`; **`super.tag()`** / **`super.init(...)`** → `get_super_method`; **`super.count = 1`** / **`super.n += 2`**; **`||=` `&&=` `??=`**; **`?.`**; **`? :`**; **`step()?`**; **`switch`+`fallthrough`**; **`do`/`while`**; **`this.run(super.f)`**; **`xs[0] += 3`**; **template `` `n=${n}` ``**; **`is(obj, "Class")`**; **`pass`/`raise`/`assert`/`not`**; **`with`/`is`/`is not`**; **`using x = expr`**; **`import.meta`**; **`delete o.x`**; **`for let i = 0; …`**; **G9** `Option.Some(42)` → `Option$Number`; **`match Option.Some(n)`** / **`Option<Number>.None`** / **`Result.Ok(n)`** / **`Result<Number, String>.Err`** kab-only; **G10** två `Option`/`Box`/`echo`/`id`-specialiseringar; **`pair$Number_String`**; **`id(id(42))`**; **`Result<Number, String>.Ok`**; **`Box<String>(…)`** explicit; **`id(b)`** → `id$Box`. **T5** ✅ — trait default-metoder emit+inject på `implements`; **`type Item = Number`** på klass (`class_assoc_types`); **`where T: Trait`** på generiska fn, metoder och klasser (`emitCheckWhere`); **`trait Show<T>`** (`interface_type_params`, `implements Show$Number`). Kvarvarande self-host-arbete: **P6b** leaf-budget ([seed/README.md](seed/README.md)). Semikolon förblir valfria.

14. **Generics fas 2 (G6–G11):** ~~G6 inferens~~ ✅, ~~G7 klassmetoder~~ ✅, ~~G8 klasser~~ ✅, ~~G9 enum~~ ✅, ~~G10 self-host~~ ✅, ~~G11 LSP~~ ✅. Plan: [docs/GENERICS.md#fas-2--g6-planering](../docs/GENERICS.md#fas-2--g6-planering), roadmap **Våg F** i [docs/ROADMAP.md](../docs/ROADMAP.md).

## Profilering (compile-tid)

Efter grön `emit` full compile — hitta flaskhalsar innan M11/M12.

```bash
# Fas-tid: parse / emit / serialize (emit.kab, kan ta timmar)
python scripts/profile_emit_compile.py phases emit.kab

# P6b leaf (minsta skip-listade källan) — samma pipeline
python scripts/profile_emit_compile.py phases self_host/serialize_body.kab

# Wall-time compile() end-to-end
python scripts/profile_emit_compile.py compile emit.kab

# Prefix-skala: vilka radintervall dominerar
python scripts/profile_emit_compile.py bisect emit

# Jämför lexer / parser / emit
python scripts/profile_emit_compile.py compare

# Run-fas (kräver _emit_full_out.kbc)
CARGO_TARGET_DIR=target-alt3 cargo test --test self_host self_host_emit_profile_run_phases -- --ignored --nocapture
```

Output-rader `PROFILE ...` är maskinläsbara. `popStack()` och stack-trim-loopar använder nu native `pop()` (kräver ny `compile(emit.kab)` för `.kbc`).

Snabb smoke: `cargo test --test self_host self_host_profile_phases_smoke`.

### P6b (skip-list → tom lista)

Se [seed/README.md](seed/README.md) för policy, playbook, **fas-profil** och baslinjer.

- Produktpath = committed seeds; **töm inte** listan förrän alla fem löv
  `compile_source_self_host` < 10 s (`P6_SELF_HOST_LEAF_CI_FAST_MS`).
- Fas-profil (mid AccAdd): parse ≈ 37% | emit ≈ 48% | serialize ≈ 15%. Landade cuts:
  maps/`emitSym`, iterative compare, **`eIfDepth`/`eMemberDepth`/`eIndexDepth`**,
  CallArg/obj/arr + callee/block depth, early `IDENT=`, `eOpsN` patches, IR + AccAdd densify.
- Leaf densify plateau → host-VM **`Rc` Array/Object** (COW + cycle reject) + Len/IndexGet.
  `serialize_body` **~144 s** debug — still ≫ 10 s, **skip-list stays**.
- Efter `emit_impl` / `parser_impl` / `serialize_body`-ändring: regenerera motsvarande `self_host/seed/*.kbc`.
