//! Kv8 optimizations — hot-path predictor, caches, scope bridge for JIT.

use super::ast::{Expr, LValue, Stmt};
use super::bytecode_bridge::Kv8BytecodeFn;
use super::context::{Kv8Context, Kv8Value};
use crate::runtime::kstyle::ComputedStyle;
use crate::value::{Environment, Value};
use std::collections::{HashMap, HashSet};

/// FNV-1a hash for script / arrow cache keys.
pub fn hash_bytes(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in data {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub fn hash_str(s: &str) -> u64 {
    hash_bytes(s.as_bytes())
}

pub fn arrow_cache_key(params: &[String], body: &Expr) -> u64 {
    hash_str(&format!("{}:{}", params.join(","), expr_key(body)))
}

pub fn expr_key(expr: &Expr) -> String {
    match expr {
        Expr::Lit(v) => super::ast::literal_to_string(v),
        Expr::Var(n) => n.clone(),
        Expr::Member(b, f) => format!("{}.{}", expr_key(b), f),
        Expr::Call(c, args) => {
            let a = args.iter().map(expr_key).collect::<Vec<_>>().join(",");
            format!("{}({a})", expr_key(c))
        }
        Expr::Bin(l, op, r) => format!("({}{}{})", expr_key(l), op, expr_key(r)),
        Expr::Unary(op, i) => format!("({op}{})", expr_key(i)),
        Expr::Arrow(p, b) => format!("({})=>{}", p.join(","), expr_key(b)),
        Expr::Block(stmts) => format!("{{{}}}", stmts.iter().map(stmt_key).collect::<Vec<_>>().join(";")),
    }
}

pub fn stmt_key(stmt: &Stmt) -> String {
    match stmt {
        Stmt::Let(n, e) => format!("let {n}={}", expr_key(e)),
        Stmt::Assign(lv, e) => format!("{}={}", lvalue_key(lv), expr_key(e)),
        Stmt::Return(e) => format!("return {}", expr_key(e)),
        Stmt::Expr(e) => expr_key(e),
        Stmt::If(c, t, e) => format!(
            "if({}){{{}}}{}",
            expr_key(c),
            t.iter().map(stmt_key).collect::<Vec<_>>().join(";"),
            e.as_ref()
                .map(|b| format!("else{{{}}}", b.iter().map(stmt_key).collect::<Vec<_>>().join(";")))
                .unwrap_or_default()
        ),
        Stmt::For(v, s, c, st, b) => format!(
            "for({v}={};{};{v}={}){{{}}}",
            expr_key(s),
            expr_key(c),
            expr_key(st),
            b.iter().map(stmt_key).collect::<Vec<_>>().join(";")
        ),
        Stmt::Function(n, p, b) => format!(
            "fn {n}({}){{{}}}",
            p.join(","),
            b.iter().map(stmt_key).collect::<Vec<_>>().join(";")
        ),
    }
}

fn lvalue_key(lv: &LValue) -> String {
    match lv {
        LValue::Name(n) => n.clone(),
        LValue::Member(b, f) => format!("{}.{}", lvalue_key(b), f),
    }
}

pub fn collect_vars_stmts(stmts: &[Stmt], out: &mut HashSet<String>) {
    for s in stmts {
        collect_vars_stmt(s, out);
    }
}

fn collect_vars_stmt(stmt: &Stmt, out: &mut HashSet<String>) {
    match stmt {
        Stmt::Let(n, e) => {
            collect_vars_expr(e, out);
            out.remove(n);
        }
        Stmt::Assign(lv, e) => {
            collect_vars_lvalue(lv, out);
            collect_vars_expr(e, out);
        }
        Stmt::Return(e) | Stmt::Expr(e) => collect_vars_expr(e, out),
        Stmt::If(c, t, e) => {
            collect_vars_expr(c, out);
            collect_vars_stmts(t, out);
            if let Some(b) = e {
                collect_vars_stmts(b, out);
            }
        }
        Stmt::For(v, s, c, st, b) => {
            collect_vars_expr(s, out);
            collect_vars_expr(c, out);
            collect_vars_expr(st, out);
            out.remove(v);
            collect_vars_stmts(b, out);
        }
        Stmt::Function(_, p, b) => {
            for param in p {
                out.remove(param);
            }
            collect_vars_stmts(b, out);
        }
    }
}

fn collect_vars_lvalue(lv: &LValue, out: &mut HashSet<String>) {
    match lv {
        LValue::Name(n) => {
            out.insert(n.clone());
        }
        LValue::Member(b, _) => collect_vars_lvalue(b, out),
    }
}

fn collect_vars_expr(expr: &Expr, out: &mut HashSet<String>) {
    match expr {
        Expr::Var(n) => {
            if n != "document" && n != "console" {
                out.insert(n.clone());
            }
        }
        Expr::Member(b, _) => collect_vars_expr(b, out),
        Expr::Call(c, args) => {
            collect_vars_expr(c, out);
            for a in args {
                collect_vars_expr(a, out);
            }
        }
        Expr::Bin(l, _, r) => {
            collect_vars_expr(l, out);
            collect_vars_expr(r, out);
        }
        Expr::Unary(_, i) => collect_vars_expr(i, out),
        Expr::Arrow(p, b) => {
            for param in p {
                out.remove(param);
            }
            collect_vars_expr(b, out);
        }
        Expr::Block(stmts) => collect_vars_stmts(stmts, out),
        Expr::Lit(_) => {}
    }
}

pub fn kv8_to_kabootar(v: &Kv8Value) -> Value {
    match v {
        Kv8Value::Undefined | Kv8Value::Null => Value::Null,
        Kv8Value::Bool(b) => Value::Bool(*b),
        Kv8Value::Num(n) => Value::Number(*n as i64),
        Kv8Value::Str(s) => Value::String(s.clone()),
        _ => Value::Null,
    }
}

pub fn sync_scope_to_env(ctx: &Kv8Context, names: &HashSet<String>, env: &mut Environment) -> Result<(), String> {
    ctx.with_mut(|inner| {
        for name in names {
            if let Some(v) = inner.scope.get(name) {
                env.set(name.clone(), kv8_to_kabootar(v));
            }
        }
        Ok(())
    })
}

pub fn sync_env_to_scope(ctx: &Kv8Context, names: &HashSet<String>, env: &Environment) -> Result<(), String> {
    ctx.with_mut(|inner| {
        for name in names {
            if let Some(v) = env.get(name) {
                inner.scope.insert(name.clone(), super::eval::kabootar_to_kv8(v));
            }
        }
        Ok(())
    })
}

/// Hot-path predictor — records monomorphic member/call sites.
#[derive(Default)]
pub struct HotPathPredictor {
    pub member_hits: HashMap<(String, String), u64>,
    pub call_hits: HashMap<String, u64>,
}

impl HotPathPredictor {
    pub fn record_member(&mut self, base: &str, field: &str) -> u64 {
        let key = (base.to_string(), field.to_string());
        let c = self.member_hits.entry(key).or_insert(0);
        *c += 1;
        *c
    }

    pub fn record_call(&mut self, native: &str) -> u64 {
        let c = self.call_hits.entry(native.to_string()).or_insert(0);
        *c += 1;
        *c
    }

    pub fn hot_members(&self) -> usize {
        self.member_hits.values().filter(|&&n| n >= 4).count()
    }

    pub fn hot_calls(&self) -> usize {
        self.call_hits.values().filter(|&&n| n >= 4).count()
    }
}

#[derive(Default)]
pub struct Kv8OptState {
    pub predictor: HotPathPredictor,
    pub program_cache: HashMap<u64, super::ast::Kv8Program>,
    pub arrow_cache: HashMap<u64, Kv8BytecodeFn>,
    pub style_cache: HashMap<u64, ComputedStyle>,
    pub style_generation: u64,
    pub dom_paths: HashMap<u64, Vec<usize>>,
    pub dom_index_dirty: bool,
    pub document_singleton: Option<Kv8Value>,
    pub console_singleton: Option<Kv8Value>,
}

impl std::fmt::Debug for Kv8OptState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Kv8OptState")
            .field("program_cache", &self.program_cache.len())
            .field("arrow_cache", &self.arrow_cache.len())
            .field("style_cache", &self.style_cache.len())
            .field("dom_paths", &self.dom_paths.len())
            .field("dom_index_dirty", &self.dom_index_dirty)
            .finish()
    }
}

pub fn rebuild_dom_index(root: &crate::runtime::kabootar_dom::DomNode, map: &mut HashMap<u64, Vec<usize>>) {
    map.clear();
    fn walk(node: &crate::runtime::kabootar_dom::DomNode, path: &mut Vec<usize>, map: &mut HashMap<u64, Vec<usize>>) {
        map.insert(node.id, path.clone());
        for (i, child) in node.children.iter().enumerate() {
            path.push(i);
            walk(child, path, map);
            path.pop();
        }
    }
    let mut path = Vec::new();
    walk(root, &mut path, map);
}

pub fn find_mut_by_path<'a>(
    root: &'a mut crate::runtime::kabootar_dom::DomNode,
    path: &[usize],
) -> Option<&'a mut crate::runtime::kabootar_dom::DomNode> {
    let mut node = root;
    for &idx in path {
        node = node.children.get_mut(idx)?;
    }
    Some(node)
}
