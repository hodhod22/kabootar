//! P13 — Cranelift JIT for typed i64 bytecode (host only).

use super::types::{BytecodeFnDef, Constant, Opcode};
use crate::value::Value;
use std::sync::atomic::{AtomicU64, Ordering};

static JIT_HITS: AtomicU64 = AtomicU64::new(0);
static JIT_COMPILED: AtomicU64 = AtomicU64::new(0);
static JIT_FAILS: AtomicU64 = AtomicU64::new(0);

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

#[cfg(not(target_arch = "wasm32"))]
mod host {
    use super::{BytecodeFnDef, Constant, Opcode, JIT_COMPILED, JIT_FAILS, JIT_HITS};
    use crate::value::Value as KabVal;
    use cranelift::codegen;
    use cranelift::prelude::*;
    use cranelift_jit::{JITBuilder, JITModule};
    use cranelift_module::{Linkage, Module};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::hash::{Hash, Hasher};
    use std::sync::atomic::Ordering;

    struct State {
        module: JITModule,
        ctx: codegen::Context,
        builder_ctx: FunctionBuilderContext,
        cache: HashMap<u64, *const u8>,
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
            add_loop: None,
        })
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
        if func.params.len() > 1 || func.locals.len() > 32 {
            return None;
        }
        if func.code.iter().any(|op| {
            matches!(
                op,
                Opcode::Div | Opcode::Mod | Opcode::Dup | Opcode::Pop | Opcode::Not | Opcode::Neg
            )
        }) {
            return None;
        }
        let key = fingerprint(func);
        let argc = func.params.len();
        let ptr = match with_state(|st| {
            if let Some(p) = st.cache.get(&key).copied() {
                return Ok(p);
            }
            let p = compile_fn(st, func, argc)?;
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
                | Opcode::AccAddLocal(i) => i.hash(&mut h),
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
        let vars: Vec<Variable> = (0..nloc).map(Variable::new).collect();
        for v in &vars {
            b.declare_var(*v, types::I64);
        }
        b.switch_to_block(entry);
        b.seal_block(entry);
        let z = b.ins().iconst(types::I64, 0);
        for v in &vars {
            b.def_var(*v, z);
        }
        for (pi, pname) in func.params.iter().enumerate() {
            if let Some(idx) = func.locals.iter().position(|l| l == pname) {
                let p = b.block_params(entry)[pi];
                b.def_var(vars[idx], p);
            }
        }
        let mut stack: Vec<cranelift::prelude::Value> = Vec::new();
        let mut terminated = false;
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
                Opcode::AccAddLocal(idx) => {
                    let rhs = stack.pop().unwrap_or(z);
                    let lhs = b.use_var(vars[idx as usize]);
                    let sum = b.ins().iadd(lhs, rhs);
                    b.def_var(vars[idx as usize], sum);
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
