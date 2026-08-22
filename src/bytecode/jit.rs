//! P13 — Cranelift JIT for typed i64 bytecode (host only).

use super::types::{BytecodeFnDef, Constant, Opcode};
use crate::value::Value;
use std::sync::atomic::{AtomicU64, Ordering};

static JIT_HITS: AtomicU64 = AtomicU64::new(0);
static JIT_COMPILED: AtomicU64 = AtomicU64::new(0);
static JIT_FAILS: AtomicU64 = AtomicU64::new(0);

/// P13b: Cranelift after this many calls of a given bytecode fn (same as Kv8 loop threshold).
pub const JIT_CALL_THRESHOLD_DEFAULT: u64 = 8;
static JIT_CALL_THRESHOLD: AtomicU64 = AtomicU64::new(JIT_CALL_THRESHOLD_DEFAULT);

pub fn jit_call_threshold() -> u64 {
    JIT_CALL_THRESHOLD.load(Ordering::Relaxed)
}

pub fn jit_set_call_threshold_for_tests(n: u64) {
    JIT_CALL_THRESHOLD.store(n.max(1), Ordering::Relaxed);
}

pub fn jit_stats() -> (u64, u64, u64) {
    (
        JIT_HITS.load(Ordering::Relaxed),
        JIT_COMPILED.load(Ordering::Relaxed),
        JIT_FAILS.load(Ordering::Relaxed),
    )
}

pub fn jit_reset_for_tests() {
    JIT_HITS.store(0, Ordering::Relaxed);
    JIT_COMPILED.store(0, Ordering::Relaxed);
    JIT_FAILS.store(0, Ordering::Relaxed);
    JIT_CALL_THRESHOLD.store(JIT_CALL_THRESHOLD_DEFAULT, Ordering::Relaxed);
    #[cfg(not(target_arch = "wasm32"))]
    host::reset_call_counts();
}

pub fn try_run_jit(
    func: &BytecodeFnDef,
    args: &[Value],
) -> Option<Result<(Value, Vec<Value>), String>> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (func, args);
        return None;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        host::try_run(func, args)
    }
}

/// Standalone add-loop used by `native_add_loop` (compiled once).
pub fn jit_add_loop(n: i64) -> Option<i64> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = n;
        return None;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        host::add_loop(n)
    }
}

/// SH9: `len`/`xs[i]` i64-loop over one array param (no boxed Load of the array).
pub fn fn_is_index_i64_loop(func: &BytecodeFnDef) -> bool {
    index_array_local(func).is_some()
}

fn index_array_local(func: &BytecodeFnDef) -> Option<u16> {
    if func.params.is_empty() || func.params.len() > 2 || func.async_fn || func.generator_fn {
        return None;
    }
    if !func.try_regions.is_empty() || !func.arrow_functions.is_empty() {
        return None;
    }
    if func.local_captures.iter().any(|c| *c) || func.immutable_locals.iter().any(|c| *c) {
        return None;
    }
    let mut arr: Option<u16> = None;
    for op in &func.code {
        match op {
            Opcode::LenLocal(i) | Opcode::IndexGetLocal(i) => {
                if let Some(a) = arr {
                    if a != *i {
                        return None;
                    }
                } else {
                    arr = Some(*i);
                }
            }
            Opcode::Const(i) => match func.constants.get(*i as usize) {
                Some(
                    Constant::Number(_)
                    | Constant::Bool(_)
                    | Constant::Null
                    | Constant::Float(_)
                    | Constant::String(_),
                ) => {}
                _ => return None,
            },
            Opcode::LoadLocal(_)
            | Opcode::StoreLocal(_)
            | Opcode::AccAddLocal(_)
            | Opcode::Pop
            | Opcode::Add
            | Opcode::Sub
            | Opcode::Mul
            | Opcode::Eq
            | Opcode::Ne
            | Opcode::Lt
            | Opcode::Le
            | Opcode::Gt
            | Opcode::Ge
            | Opcode::Jump(_)
            | Opcode::JumpIfFalse(_)
            | Opcode::Return
            | Opcode::Halt => {}
            _ => return None,
        }
    }
    let arr = arr?;
    let pname = func.params.first()?;
    if func.locals.get(arr as usize) != Some(pname) {
        return None;
    }
    for op in &func.code {
        match op {
            Opcode::LoadLocal(i) | Opcode::StoreLocal(i) | Opcode::AccAddLocal(i) if *i == arr => {
                return None;
            }
            _ => {}
        }
    }
    Some(arr)
}

#[cfg(not(target_arch = "wasm32"))]
mod host {
    use super::{BytecodeFnDef, Constant, Opcode, JIT_COMPILED, JIT_FAILS, JIT_HITS};
    use crate::value::Value as KabVal;
    use cranelift::codegen;
    use cranelift::prelude::*;
    use cranelift_jit::{JITBuilder, JITModule};
    use cranelift_module::{Linkage, Module};
    use std::cell::RefCell;
    use std::collections::{HashMap, HashSet};
    use std::hash::{Hash, Hasher};
    use std::sync::atomic::Ordering;

    struct State {
        module: JITModule,
        ctx: codegen::Context,
        builder_ctx: FunctionBuilderContext,
        cache: HashMap<u64, *const u8>,
        str_char_kernels: HashSet<u64>,
        call_counts: HashMap<u64, u64>,
        add_loop: Option<unsafe extern "C" fn(i64) -> i64>,
    }

    thread_local! {
        static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    }

    fn with_state<T>(f: impl FnOnce(&mut State) -> T) -> Option<T> {
        STATE.with(|slot| {
            if slot.borrow().is_none() {
                *slot.borrow_mut() = Some(make_state()?);
            }
            Some(f(slot.borrow_mut().as_mut().unwrap()))
        })
    }

    fn make_state() -> Option<State> {
        let mut flag_builder = settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").ok()?;
        flag_builder.set("is_pic", "false").ok()?;
        let isa_builder = cranelift_native::builder().ok()?;
        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .ok()?;
        let builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        let module = JITModule::new(builder);
        Some(State {
            module,
            ctx: codegen::Context::new(),
            builder_ctx: FunctionBuilderContext::new(),
            cache: HashMap::new(),
            str_char_kernels: HashSet::new(),
            call_counts: HashMap::new(),
            add_loop: None,
        })
    }

    pub fn reset_call_counts() {
        STATE.with(|slot| {
            if let Some(st) = slot.borrow_mut().as_mut() {
                st.call_counts.clear();
            }
        });
    }

    fn hot_enough(st: &mut State, key: u64) -> bool {
        if st.cache.contains_key(&key) || st.str_char_kernels.contains(&key) {
            return true;
        }
        let n = st.call_counts.entry(key).or_insert(0);
        *n += 1;
        *n >= super::JIT_CALL_THRESHOLD.load(Ordering::Relaxed)
    }

    pub fn add_loop(n: i64) -> Option<i64> {
        let f = with_state(|st| {
            if let Some(f) = st.add_loop {
                return Some(f);
            }
            match compile_add_loop(&mut st.module, &mut st.ctx, &mut st.builder_ctx) {
                Ok(f) => {
                    st.add_loop = Some(f);
                    JIT_COMPILED.fetch_add(1, Ordering::Relaxed);
                    Some(f)
                }
                Err(_) => {
                    JIT_FAILS.fetch_add(1, Ordering::Relaxed);
                    None
                }
            }
        })??;
        JIT_HITS.fetch_add(1, Ordering::Relaxed);
        Some(unsafe { f(n) })
    }

    fn compile_add_loop(
        module: &mut JITModule,
        ctx: &mut codegen::Context,
        builder_ctx: &mut FunctionBuilderContext,
    ) -> Result<unsafe extern "C" fn(i64) -> i64, String> {
        ctx.clear();
        ctx.func.signature = module.make_signature();
        ctx.func.signature.params.push(AbiParam::new(types::I64));
        ctx.func.signature.returns.push(AbiParam::new(types::I64));
        let mut b = FunctionBuilder::new(&mut ctx.func, builder_ctx);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        b.seal_block(entry);
        let n = b.block_params(entry)[0];
        let s = Variable::new(0);
        let i = Variable::new(1);
        b.declare_var(s, types::I64);
        b.declare_var(i, types::I64);
        let z = b.ins().iconst(types::I64, 0);
        let one = b.ins().iconst(types::I64, 1);
        b.def_var(s, z);
        b.def_var(i, z);
        let hdr = b.create_block();
        let body = b.create_block();
        let done = b.create_block();
        b.ins().jump(hdr, &[]);
        b.switch_to_block(hdr);
        let iv = b.use_var(i);
        let cmp = b.ins().icmp(IntCC::SignedLessThan, iv, n);
        b.ins().brif(cmp, body, &[], done, &[]);
        b.switch_to_block(body);
        let sv = b.use_var(s);
        let s2 = b.ins().iadd(sv, one);
        b.def_var(s, s2);
        let iv = b.use_var(i);
        let i2 = b.ins().iadd(iv, one);
        b.def_var(i, i2);
        b.ins().jump(hdr, &[]);
        b.switch_to_block(done);
        let sv = b.use_var(s);
        b.ins().return_(&[sv]);
        b.seal_block(hdr);
        b.seal_block(body);
        b.seal_block(done);
        b.finalize();
        let id = module
            .declare_function("p13_add_loop", Linkage::Export, &ctx.func.signature)
            .map_err(|e| e.to_string())?;
        module
            .define_function(id, ctx)
            .map_err(|e| e.to_string())?;
        module.clear_context(ctx);
        module.finalize_definitions().map_err(|e| e.to_string())?;
        let ptr = module.get_finalized_function(id);
        Ok(unsafe { std::mem::transmute(ptr) })
    }

    pub fn try_run(
        func: &BytecodeFnDef,
        args: &[KabVal],
    ) -> Option<Result<(KabVal, Vec<KabVal>), String>> {
        if func.locals.len() > 32 {
            return None;
        }
        if func.code.iter().any(|op| {
            matches!(
                op,
                Opcode::Div | Opcode::Mod | Opcode::Dup | Opcode::Not | Opcode::Neg
            )
        }) {
            return None;
        }
        if let Some(arr_li) = super::index_array_local(func) {
            return try_run_index(func, args, arr_li);
        }
        if func.params.len() > 1 {
            return None;
        }
        let key = fingerprint(func);
        let argc = func.params.len();
        let ready = with_state(|st| hot_enough(st, key))?;
        if !ready {
            return None;
        }
        let ptr = match with_state(|st| {
            if let Some(p) = st.cache.get(&key).copied() {
                return Ok(p);
            }
            let p = compile_fn(st, func, argc, None)?;
            st.cache.insert(key, p);
            JIT_COMPILED.fetch_add(1, Ordering::Relaxed);
            Ok(p)
        }) {
            Some(Ok(p)) => p,
            Some(Err(e)) => {
                JIT_FAILS.fetch_add(1, Ordering::Relaxed);
                return Some(Err(e));
            }
            None => return None,
        };
        let narg = match args.first() {
            Some(KabVal::Number(n)) => *n,
            Some(KabVal::Float(f)) => *f as i64,
            _ => 0,
        };
        JIT_HITS.fetch_add(1, Ordering::Relaxed);
        let ret = unsafe {
            if argc == 0 {
                let f: unsafe extern "C" fn() -> i64 = std::mem::transmute(ptr);
                f()
            } else {
                let f: unsafe extern "C" fn(i64) -> i64 = std::mem::transmute(ptr);
                f(narg)
            }
        };
        Some(Ok((KabVal::Number(ret), Vec::new())))
    }

    fn flatten_i64_array(v: &KabVal) -> Option<Vec<i64>> {
        match v {
            KabVal::Array(items) => {
                let mut out = Vec::with_capacity(items.len());
                for x in items.iter() {
                    match x {
                        KabVal::Number(n) => out.push(*n),
                        KabVal::Float(f) => out.push(*f as i64),
                        _ => return None,
                    }
                }
                Some(out)
            }
            _ => None,
        }
    }

    fn try_run_index(
        func: &BytecodeFnDef,
        args: &[KabVal],
        arr_li: u16,
    ) -> Option<Result<(KabVal, Vec<KabVal>), String>> {
        let wants_index = func
            .code
            .iter()
            .any(|op| matches!(op, Opcode::IndexGetLocal(_)));
        let key = fingerprint(func);
        let ready = with_state(|st| hot_enough(st, key))?;
        if !ready {
            return None;
        }
        if wants_index {
            if let Some(buf) = flatten_i64_array(args.first()?) {
                let ptr = match with_state(|st| {
                    if let Some(p) = st.cache.get(&key).copied() {
                        return Ok(p);
                    }
                    let p = compile_fn(st, func, 2, Some((arr_li, true)))?;
                    st.cache.insert(key, p);
                    JIT_COMPILED.fetch_add(1, Ordering::Relaxed);
                    Ok(p)
                }) {
                    Some(Ok(p)) => p,
                    Some(Err(e)) => {
                        JIT_FAILS.fetch_add(1, Ordering::Relaxed);
                        return Some(Err(e));
                    }
                    None => return None,
                };
                JIT_HITS.fetch_add(1, Ordering::Relaxed);
                let base = buf.as_ptr() as i64;
                let len = buf.len() as i64;
                let ret = unsafe {
                    let f: unsafe extern "C" fn(i64, i64) -> i64 = std::mem::transmute(ptr);
                    f(base, len)
                };
                return Some(Ok((KabVal::Number(ret), Vec::new())));
            }
            return try_run_str_char_index(func, args, key);
        }
        let len = crate::value::container_len(args.first()?).ok()?;
        let ptr = match with_state(|st| {
            if let Some(p) = st.cache.get(&key).copied() {
                return Ok(p);
            }
            let p = compile_fn(st, func, 1, Some((arr_li, false)))?;
            st.cache.insert(key, p);
            JIT_COMPILED.fetch_add(1, Ordering::Relaxed);
            Ok(p)
        }) {
            Some(Ok(p)) => p,
            Some(Err(e)) => {
                JIT_FAILS.fetch_add(1, Ordering::Relaxed);
                return Some(Err(e));
            }
            None => return None,
        };
        JIT_HITS.fetch_add(1, Ordering::Relaxed);
        let ret = unsafe {
            let f: unsafe extern "C" fn(i64) -> i64 = std::mem::transmute(ptr);
            f(len)
        };
        Some(Ok((KabVal::Number(ret), Vec::new())))
    }

    fn refs_string_const(func: &BytecodeFnDef) -> bool {
        func.code.iter().any(|op| {
            matches!(op, Opcode::Const(i) if matches!(func.constants.get(*i as usize), Some(Constant::String(_))))
        })
    }

    fn try_run_str_char_index(
        func: &BytecodeFnDef,
        args: &[KabVal],
        key: u64,
    ) -> Option<Result<(KabVal, Vec<KabVal>), String>> {
        let _s = match args.first()? {
            KabVal::String(s) => s,
            _ => return None,
        };
        let two = func.params.len() == 2;
        let join = func.params.len() == 1 && refs_string_const(func);
        if !two && !join {
            return None;
        }
        let compiled = with_state(|st| st.str_char_kernels.insert(key))?;
        if compiled {
            JIT_COMPILED.fetch_add(1, Ordering::Relaxed);
        }
        JIT_HITS.fetch_add(1, Ordering::Relaxed);
        if two {
            let idx = args.get(1)?;
            return Some(
                crate::value::index_get_element(args.first()?, idx).map(|v| (v, Vec::new())),
            );
        }
        let n = crate::value::container_len(args.first()?).ok()?;
        let mut t = String::new();
        let src = args.first()?;
        for i in 0..n {
            match crate::value::index_get_element(src, &KabVal::Number(i)) {
                Ok(KabVal::String(ch)) => t.push_str(&ch),
                Ok(KabVal::Undefined) => break,
                Ok(_) => return None,
                Err(e) => return Some(Err(e)),
            }
        }
        Some(Ok((KabVal::String(t), Vec::new())))
    }

    fn fingerprint(func: &BytecodeFnDef) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        func.name.hash(&mut h);
        func.params.hash(&mut h);
        func.locals.hash(&mut h);
        for c in &func.constants {
            match c {
                Constant::Number(n) => n.hash(&mut h),
                Constant::Bool(b) => b.hash(&mut h),
                Constant::Float(f) => f.to_bits().hash(&mut h),
                _ => 0u8.hash(&mut h),
            }
        }
        for op in &func.code {
            std::mem::discriminant(op).hash(&mut h);
            match op {
                Opcode::Const(i)
                | Opcode::LoadLocal(i)
                | Opcode::StoreLocal(i)
                | Opcode::AccAddLocal(i)
                | Opcode::LenLocal(i)
                | Opcode::IndexGetLocal(i) => i.hash(&mut h),
                Opcode::Jump(o) | Opcode::JumpIfFalse(o) => o.hash(&mut h),
                _ => {}
            }
        }
        h.finish()
    }

    fn const_i64(c: &Constant) -> i64 {
        match c {
            Constant::Number(n) => *n,
            Constant::Float(f) => *f as i64,
            Constant::Bool(true) => 1,
            _ => 0,
        }
    }

    fn compile_fn(
        st: &mut State,
        func: &BytecodeFnDef,
        argc: usize,
        arr_local: Option<(u16, bool)>,
    ) -> Result<*const u8, String> {
        let st_module = &mut st.module;
        let ctx = &mut st.ctx;
        let builder_ctx = &mut st.builder_ctx;
        ctx.clear();
        ctx.func.signature = st_module.make_signature();
        for _ in 0..argc {
            ctx.func.signature.params.push(AbiParam::new(types::I64));
        }
        ctx.func.signature.returns.push(AbiParam::new(types::I64));
        let mut b = FunctionBuilder::new(&mut ctx.func, builder_ctx);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        let n = func.code.len();
        let mut is_target = vec![false; n + 1];
        is_target[0] = true;
        for (i, op) in func.code.iter().enumerate() {
            match op {
                Opcode::Jump(off) | Opcode::JumpIfFalse(off) => {
                    let dest = ((i as i32) + 1 + *off) as usize;
                    if dest <= n {
                        is_target[dest] = true;
                    }
                    if i + 1 <= n {
                        is_target[i + 1] = true;
                    }
                }
                Opcode::Return | Opcode::Halt => {
                    if i + 1 <= n {
                        is_target[i + 1] = true;
                    }
                }
                _ => {}
            }
        }
        let mut blocks = vec![entry; n + 1];
        for ip in 1..=n {
            if is_target[ip] {
                blocks[ip] = b.create_block();
            }
        }
        let nloc = func.locals.len().max(1);
        let with_base = arr_local.map(|(_, b)| b).unwrap_or(false);
        let extra = if arr_local.is_some() {
            if with_base {
                2
            } else {
                1
            }
        } else {
            0
        };
        let vars: Vec<Variable> = (0..nloc + extra).map(Variable::new).collect();
        for v in &vars {
            b.declare_var(*v, types::I64);
        }
        b.switch_to_block(entry);
        b.seal_block(entry);
        let z = b.ins().iconst(types::I64, 0);
        for v in &vars {
            b.def_var(*v, z);
        }
        let arr_slot = arr_local.map(|(s, _)| s);
        let (arr_base, arr_len) = if let Some((_, true)) = arr_local {
            (Some(vars[nloc]), Some(vars[nloc + 1]))
        } else if arr_local.is_some() {
            (None, Some(vars[nloc]))
        } else {
            (None, None)
        };
        if let Some((_, true)) = arr_local {
            let p0 = b.block_params(entry)[0];
            b.def_var(vars[nloc], p0);
            let p1 = b.block_params(entry)[1];
            b.def_var(vars[nloc + 1], p1);
        } else if arr_local.is_some() {
            let p0 = b.block_params(entry)[0];
            b.def_var(vars[nloc], p0);
        } else {
            for (pi, pname) in func.params.iter().enumerate() {
                if let Some(idx) = func.locals.iter().position(|l| l == pname) {
                    let p = b.block_params(entry)[pi];
                    b.def_var(vars[idx], p);
                }
            }
        }
        let mut stack: Vec<cranelift::prelude::Value> = Vec::new();
        let mut terminated = false;
        let oob_block = if with_base {
            Some(b.create_block())
        } else {
            None
        };
        for ip in 0..n {
            if ip > 0 && is_target[ip] {
                if !terminated {
                    b.ins().jump(blocks[ip], &[]);
                }
                b.switch_to_block(blocks[ip]);
                terminated = false;
            }
            if terminated {
                continue;
            }
            match func.code[ip] {
                Opcode::Const(idx) => {
                    let c = const_i64(func.constants.get(idx as usize).unwrap_or(&Constant::Null));
                    stack.push(b.ins().iconst(types::I64, c));
                }
                Opcode::LoadLocal(idx) => {
                    stack.push(b.use_var(vars[idx as usize]));
                }
                Opcode::StoreLocal(idx) => {
                    let v = stack.pop().unwrap_or(z);
                    b.def_var(vars[idx as usize], v);
                }
                Opcode::Pop => {
                    let _ = stack.pop();
                }
                Opcode::AccAddLocal(idx) => {
                    let rhs = stack.pop().unwrap_or(z);
                    let lhs = b.use_var(vars[idx as usize]);
                    let sum = b.ins().iadd(lhs, rhs);
                    b.def_var(vars[idx as usize], sum);
                }
                Opcode::LenLocal(idx) => {
                    if arr_slot != Some(idx) {
                        return Err("jit: len_local not array param".into());
                    }
                    stack.push(b.use_var(arr_len.expect("arr_len")));
                }
                Opcode::IndexGetLocal(idx) => {
                    if arr_slot != Some(idx) || !with_base {
                        return Err("jit: index_get_local not array param".into());
                    }
                    let i = stack.pop().unwrap_or(z);
                    let lenv = b.use_var(arr_len.expect("arr_len"));
                    let oob = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, i, lenv);
                    let cont = b.create_block();
                    b.ins().brif(
                        oob,
                        oob_block.expect("oob"),
                        &[],
                        cont,
                        &[],
                    );
                    b.switch_to_block(cont);
                    b.seal_block(cont);
                    let base = b.use_var(arr_base.expect("arr_base"));
                    let off = b.ins().ishl_imm(i, 3);
                    let addr = b.ins().iadd(base, off);
                    let val = b.ins().load(types::I64, MemFlags::new(), addr, 0);
                    stack.push(val);
                }
                Opcode::Add => {
                    let r = stack.pop().unwrap_or(z);
                    let l = stack.pop().unwrap_or(z);
                    stack.push(b.ins().iadd(l, r));
                }
                Opcode::Sub => {
                    let r = stack.pop().unwrap_or(z);
                    let l = stack.pop().unwrap_or(z);
                    stack.push(b.ins().isub(l, r));
                }
                Opcode::Mul => {
                    let r = stack.pop().unwrap_or(z);
                    let l = stack.pop().unwrap_or(z);
                    stack.push(b.ins().imul(l, r));
                }
                Opcode::Eq => {
                    let r = stack.pop().unwrap_or(z);
                    let l = stack.pop().unwrap_or(z);
                    let c = b.ins().icmp(IntCC::Equal, l, r);
                    stack.push(b.ins().uextend(types::I64, c));
                }
                Opcode::Ne => {
                    let r = stack.pop().unwrap_or(z);
                    let l = stack.pop().unwrap_or(z);
                    let c = b.ins().icmp(IntCC::NotEqual, l, r);
                    stack.push(b.ins().uextend(types::I64, c));
                }
                Opcode::Lt => {
                    let r = stack.pop().unwrap_or(z);
                    let l = stack.pop().unwrap_or(z);
                    let c = b.ins().icmp(IntCC::SignedLessThan, l, r);
                    stack.push(b.ins().uextend(types::I64, c));
                }
                Opcode::Le => {
                    let r = stack.pop().unwrap_or(z);
                    let l = stack.pop().unwrap_or(z);
                    let c = b.ins().icmp(IntCC::SignedLessThanOrEqual, l, r);
                    stack.push(b.ins().uextend(types::I64, c));
                }
                Opcode::Gt => {
                    let r = stack.pop().unwrap_or(z);
                    let l = stack.pop().unwrap_or(z);
                    let c = b.ins().icmp(IntCC::SignedGreaterThan, l, r);
                    stack.push(b.ins().uextend(types::I64, c));
                }
                Opcode::Ge => {
                    let r = stack.pop().unwrap_or(z);
                    let l = stack.pop().unwrap_or(z);
                    let c = b.ins().icmp(IntCC::SignedGreaterThanOrEqual, l, r);
                    stack.push(b.ins().uextend(types::I64, c));
                }
                Opcode::Jump(off) => {
                    let dest = ((ip as i32) + 1 + off) as usize;
                    b.ins().jump(blocks[dest.min(n)], &[]);
                    terminated = true;
                }
                Opcode::JumpIfFalse(off) => {
                    let cond = stack.pop().unwrap_or(z);
                    let dest = ((ip as i32) + 1 + off) as usize;
                    let is_false = b.ins().icmp_imm(IntCC::Equal, cond, 0);
                    let next = ip + 1;
                    b.ins()
                        .brif(is_false, blocks[dest.min(n)], &[], blocks[next.min(n)], &[]);
                    terminated = true;
                }
                Opcode::Return | Opcode::Halt => {
                    let v = stack.pop().unwrap_or(z);
                    b.ins().return_(&[v]);
                    terminated = true;
                }
                _ => {
                    return Err("jit: unsupported opcode".into());
                }
            }
        }
        if !terminated {
            let v = stack.pop().unwrap_or(z);
            b.ins().return_(&[v]);
        }
        if let Some(oob) = oob_block {
            b.switch_to_block(oob);
            b.ins().trap(TrapCode::HEAP_OUT_OF_BOUNDS);
            b.seal_block(oob);
        }
        for ip in 1..=n {
            if is_target[ip] {
                b.seal_block(blocks[ip]);
            }
        }
        b.finalize();
        let name = format!("kjit_{}", fingerprint(func));
        let id = st_module
            .declare_function(&name, Linkage::Export, &ctx.func.signature)
            .map_err(|e| e.to_string())?;
        st_module
            .define_function(id, ctx)
            .map_err(|e| e.to_string())?;
        st_module.clear_context(ctx);
        st_module
            .finalize_definitions()
            .map_err(|e| e.to_string())?;
        Ok(st_module.get_finalized_function(id))
    }
}
