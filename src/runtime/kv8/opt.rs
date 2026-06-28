//! Kv8 optimizations — hot-path predictor, caches, scope bridge for JIT.

use super::ast::{Expr, Kv8Param, LValue, ObjectEntryKey, Stmt};
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

pub fn arrow_cache_key(params: &[Kv8Param], body: &Expr) -> u64 {
    hash_str(&format!("{}:{}", params_key(params), expr_key(body)))
}

fn params_key(params: &[Kv8Param]) -> String {
    params
        .iter()
        .map(|(n, d)| {
            if let Some(e) = d {
                format!("{n}={}", expr_key(e))
            } else {
                n.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

pub fn expr_key(expr: &Expr) -> String {
    match expr {
        Expr::Lit(v) => super::ast::literal_to_string(v),
        Expr::Var(n) => n.clone(),
        Expr::Member(b, f) => format!("{}.{}", expr_key(b), f),
        Expr::Index(b, i) => format!("{}[{}]", expr_key(b), expr_key(i)),
        Expr::Call(c, args) => {
            let a = args.iter().map(expr_key).collect::<Vec<_>>().join(",");
            format!("{}({a})", expr_key(c))
        }
        Expr::Bin(l, op, r) => format!("({}{}{})", expr_key(l), op, expr_key(r)),
        Expr::Unary(op, i) => format!("({op}{})", expr_key(i)),
        Expr::Arrow(p, b) => format!("({})=>{}", params_key(p), expr_key(b)),
        Expr::Block(stmts) => format!("{{{}}}", stmts.iter().map(stmt_key).collect::<Vec<_>>().join(";")),
        Expr::Object(pairs) => format!(
            "{{{}}}",
            pairs
                .iter()
                .map(|(k, e)| {
                    let key = match k {
                        ObjectEntryKey::Lit(s) => s.clone(),
                        ObjectEntryKey::Computed(expr) => format!("[{}]", expr_key(expr)),
                        ObjectEntryKey::Spread(expr) => format!("...{}", expr_key(expr)),
                    };
                    format!("{key}:{}", expr_key(e))
                })
                .collect::<Vec<_>>()
                .join(",")
        ),
        Expr::Array(elems) => format!(
            "[{}]",
            elems.iter().map(expr_key).collect::<Vec<_>>().join(",")
        ),
        Expr::New(c, a) => format!(
            "new {}({})",
            expr_key(c),
            a.iter().map(expr_key).collect::<Vec<_>>().join(",")
        ),
        Expr::Await(i) => format!("await {}", expr_key(i)),
        Expr::Seq(exprs) => format!(
            "({})",
            exprs.iter().map(expr_key).collect::<Vec<_>>().join(",")
        ),
        Expr::AssignExpr(lv, op, rhs) => format!("{}{}{}", lvalue_key(lv), op, expr_key(rhs)),
        Expr::Cond(c, t, e) => format!("({})?{}:{}", expr_key(c), expr_key(t), expr_key(e)),
        Expr::Update(lv, op, prefix) => {
            if *prefix {
                format!("{}{}", op, lvalue_key(lv))
            } else {
                format!("{}{}", lvalue_key(lv), op)
            }
        }
        Expr::FunExpr(params, body) => format!(
            "fn kv8_fun_expr({}){{{}}}",
            params_key(params),
            body.iter().map(stmt_key).collect::<Vec<_>>().join(";")
        ),
        Expr::OptMember(b, f) => format!("{}?.{}", expr_key(b), f),
        Expr::OptIndex(b, i) => format!("{}?.[{}]", expr_key(b), expr_key(i)),
        Expr::OptCall(c, args) => {
            let a = args.iter().map(expr_key).collect::<Vec<_>>().join(",");
            format!("{}?.({a})", expr_key(c))
        }
        Expr::Template(parts) => format!(
            "`{}`",
            parts
                .iter()
                .map(|p| match p {
                    super::ast::TemplatePart::Lit(s) => s.clone(),
                    super::ast::TemplatePart::Expr(e) => format!("${{{}}}", expr_key(e)),
                })
                .collect::<Vec<_>>()
                .join("")
        ),
        Expr::This => "this".into(),
    }
}

pub fn stmt_key(stmt: &Stmt) -> String {
    match stmt {
        Stmt::Var(n, e) => format!("var {n}={}", expr_key(e)),
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
        Stmt::While(c, b) => format!(
            "while({}){{{}}}",
            expr_key(c),
            b.iter().map(stmt_key).collect::<Vec<_>>().join(";")
        ),
        Stmt::Break(l) => l
            .as_ref()
            .map(|n| format!("break {n}"))
            .unwrap_or_else(|| "break".into()),
        Stmt::Continue(l) => l
            .as_ref()
            .map(|n| format!("continue {n}"))
            .unwrap_or_else(|| "continue".into()),
        Stmt::Label(n, inner) => format!("{n}:{}", stmt_key(inner)),
        Stmt::Block(stmts) => format!(
            "{{{}}}",
            stmts.iter().map(stmt_key).collect::<Vec<_>>().join(";")
        ),
        Stmt::DoWhile(body, cond) => format!(
            "do{{{}}}while({})",
            body.iter().map(stmt_key).collect::<Vec<_>>().join(";"),
            expr_key(cond)
        ),
        Stmt::Throw(e) => format!("throw {}", expr_key(e)),
        Stmt::Switch(d, cases, def) => format!(
            "switch({}){{{}{}}}",
            expr_key(d),
            cases
                .iter()
                .map(|c| {
                    format!(
                        "case {}:{}",
                        expr_key(&c.label),
                        c.body.iter().map(stmt_key).collect::<Vec<_>>().join(";")
                    )
                })
                .collect::<Vec<_>>()
                .join(""),
            def.as_ref()
                .map(|b| format!("default:{}", b.iter().map(stmt_key).collect::<Vec<_>>().join(";")))
                .unwrap_or_default()
        ),
        Stmt::ForClassic(init, _cond, _update, body) => format!(
            "for({}){{{}}}",
            init.iter().map(stmt_key).collect::<Vec<_>>().join(";"),
            body.iter().map(stmt_key).collect::<Vec<_>>().join(";")
        ),
        Stmt::ForIn(lv, iter, body) => format!(
            "for({} in {}){{{}}}",
            lvalue_key(lv),
            expr_key(iter),
            body.iter().map(stmt_key).collect::<Vec<_>>().join(";")
        ),
        Stmt::ForOf(lv, iter, body) => format!(
            "for({} of {}){{{}}}",
            lvalue_key(lv),
            expr_key(iter),
            body.iter().map(stmt_key).collect::<Vec<_>>().join(";")
        ),
        Stmt::Import { default, named, from } => format!(
            "import {}{}{} from \"{from}\"",
            default.as_deref().unwrap_or(""),
            if default.is_some() && !named.is_empty() {
                ", "
            } else {
                ""
            },
            named.join(", ")
        ),
        Stmt::ExportDefault(e) => format!("export default {}", expr_key(e)),
        Stmt::ExportNamed(names) => format!("export {{ {} }}", names.join(", ")),
        Stmt::TryCatch(try_b, catch, fin) => {
            let catch_part = catch
                .as_ref()
                .map(|(var, catch_b)| {
                    format!(
                        "catch({}){{{}}}",
                        var,
                        catch_b.iter().map(stmt_key).collect::<Vec<_>>().join(";")
                    )
                })
                .unwrap_or_default();
            format!(
                "try{{{}}}{}{}",
                try_b.iter().map(stmt_key).collect::<Vec<_>>().join(";"),
                catch_part,
                fin.as_ref()
                    .map(|b| format!("finally{{{}}}", b.iter().map(stmt_key).collect::<Vec<_>>().join(";")))
                    .unwrap_or_default()
            )
        }
        Stmt::Function(n, p, b) => format!(
            "fn {n}({}){{{}}}",
            params_key(p),
            b.iter().map(stmt_key).collect::<Vec<_>>().join(";")
        ),
        Stmt::AsyncFunction(n, p, b) => format!(
            "async fn {n}({}){{{}}}",
            params_key(p),
            b.iter().map(stmt_key).collect::<Vec<_>>().join(";")
        ),
    }
}

fn lvalue_key(lv: &LValue) -> String {
    match lv {
        LValue::Name(n) => n.clone(),
        LValue::This => "this".into(),
        LValue::Member(b, f) => format!("{}.{}", lvalue_key(b), f),
        LValue::Index(b, i) => format!("{}[{}]", lvalue_key(b), expr_key(i)),
        LValue::MemberExpr(b, f) => format!("({}).{}", expr_key(b), f),
        LValue::IndexExpr(b, i) => format!("({})[{}]", expr_key(b), expr_key(i)),
    }
}

pub fn collect_vars_stmts(stmts: &[Stmt], out: &mut HashSet<String>) {
    for s in stmts {
        collect_vars_stmt(s, out);
    }
}

/// `var` names in a function body — hoisted to `undefined` at function entry.
pub fn collect_var_hoists(stmts: &[Stmt], out: &mut HashSet<String>) {
    for s in stmts {
        match s {
            Stmt::Var(name, _) => {
                out.insert(name.clone());
            }
            Stmt::Block(stmts) => collect_var_hoists(stmts, out),
            Stmt::If(_, then_b, else_b) => {
                collect_var_hoists(then_b, out);
                if let Some(b) = else_b {
                    collect_var_hoists(b, out);
                }
            }
            Stmt::For(_, _, _, _, b) | Stmt::While(_, b) => collect_var_hoists(b, out),
            Stmt::ForClassic(init, _, _, b) => {
                collect_var_hoists(init, out);
                collect_var_hoists(b, out);
            }
            Stmt::ForIn(_, _, b) | Stmt::ForOf(_, _, b) => collect_var_hoists(b, out),
            Stmt::DoWhile(b, _) => collect_var_hoists(b, out),
            Stmt::TryCatch(try_b, catch, fin) => {
                collect_var_hoists(try_b, out);
                if let Some((_, catch_b)) = catch {
                    collect_var_hoists(catch_b, out);
                }
                if let Some(b) = fin {
                    collect_var_hoists(b, out);
                }
            }
            Stmt::Switch(_, cases, def) => {
                for c in cases {
                    collect_var_hoists(&c.body, out);
                }
                if let Some(b) = def {
                    collect_var_hoists(b, out);
                }
            }
            Stmt::Label(_, inner) => collect_var_hoists(std::slice::from_ref(inner), out),
            Stmt::Function(_, _, body) | Stmt::AsyncFunction(_, _, body) => {
                collect_var_hoists(body, out);
            }
            _ => {}
        }
    }
}

/// Loop bodies with `break`/`continue` must stay on the interpreter path (JIT wraps body in a fn).
pub fn stmts_have_loop_control(stmts: &[Stmt]) -> bool {
    stmts.iter().any(stmt_has_loop_control)
}

fn stmt_has_loop_control(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Break(_) | Stmt::Continue(_) => true,
        Stmt::Label(_, inner) => stmt_has_loop_control(inner),
        Stmt::Block(stmts) => stmts_have_loop_control(stmts),
        Stmt::DoWhile(body, cond) => stmts_have_loop_control(body) || expr_has_loop_control(cond),
        Stmt::Throw(_) => false,
        Stmt::If(_, t, e) => {
            stmts_have_loop_control(t) || e.as_ref().is_some_and(|b| stmts_have_loop_control(b))
        }
        Stmt::For(_, _, _, _, b) | Stmt::While(_, b) | Stmt::ForClassic(_, _, _, b) => {
            stmts_have_loop_control(b)
        }
        Stmt::Switch(d, cases, def) => {
            expr_has_loop_control(d)
                || cases.iter().any(|c| {
                    expr_has_loop_control(&c.label) || stmts_have_loop_control(&c.body)
                })
                || def.as_ref().is_some_and(|b| stmts_have_loop_control(b))
        }
        Stmt::ForIn(_, iter, b) | Stmt::ForOf(_, iter, b) => {
            expr_has_loop_control(iter) || stmts_have_loop_control(b)
        }
        Stmt::Import { .. } | Stmt::ExportDefault(_) | Stmt::ExportNamed(_) => false,
        Stmt::TryCatch(try_b, catch, fin) => {
            stmts_have_loop_control(try_b)
                || catch
                    .as_ref()
                    .is_some_and(|(_, catch_b)| stmts_have_loop_control(catch_b))
                || fin.as_ref().is_some_and(|b| stmts_have_loop_control(b))
        }
        Stmt::Function(_, _, b) | Stmt::AsyncFunction(_, _, b) => stmts_have_loop_control(b),
        Stmt::Var(_, e) | Stmt::Let(_, e) | Stmt::Return(e) | Stmt::Expr(e) | Stmt::Assign(_, e) => {
            expr_has_loop_control(e)
        }
    }
}

fn expr_has_loop_control(expr: &Expr) -> bool {
    match expr {
        Expr::Block(stmts) => stmts_have_loop_control(stmts),
        Expr::Arrow(_, body) => expr_has_loop_control(body),
        Expr::Member(b, _) | Expr::OptMember(b, _) => expr_has_loop_control(b),
        Expr::Index(b, i) | Expr::OptIndex(b, i) => {
            expr_has_loop_control(b) || expr_has_loop_control(i)
        }
        Expr::Call(c, args) | Expr::OptCall(c, args) => {
            expr_has_loop_control(c) || args.iter().any(expr_has_loop_control)
        }
        Expr::Bin(l, _, r) => expr_has_loop_control(l) || expr_has_loop_control(r),
        Expr::Unary(_, i) | Expr::Await(i) | Expr::New(i, _) => expr_has_loop_control(i),
        Expr::Object(pairs) => pairs.iter().any(|(_, e)| expr_has_loop_control(e)),
        Expr::Array(elems) => elems.iter().any(expr_has_loop_control),
        Expr::Seq(exprs) => exprs.iter().any(expr_has_loop_control),
        Expr::AssignExpr(_, _, rhs) => expr_has_loop_control(rhs),
        Expr::Update(_, _, _) => false,
        Expr::Cond(c, t, e) => {
            expr_has_loop_control(c) || expr_has_loop_control(t) || expr_has_loop_control(e)
        }
        Expr::FunExpr(_, body) => stmts_have_loop_control(body),
        Expr::Template(parts) => parts.iter().any(|p| {
            matches!(p, super::ast::TemplatePart::Expr(e) if expr_has_loop_control(e))
        }),
        Expr::This | Expr::Lit(_) | Expr::Var(_) => false,
    }
}

fn collect_vars_stmt(stmt: &Stmt, out: &mut HashSet<String>) {
    match stmt {
        Stmt::Var(n, e) | Stmt::Let(n, e) => {
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
        Stmt::ForClassic(init, cond, update, b) => {
            collect_vars_stmts(init, out);
            if let Some(c) = cond {
                collect_vars_expr(c, out);
            }
            if let Some(u) = update {
                collect_vars_expr(u, out);
            }
            collect_vars_stmts(b, out);
        }
        Stmt::ForIn(lv, iter, b) | Stmt::ForOf(lv, iter, b) => {
            collect_vars_lvalue(lv, out);
            collect_vars_expr(iter, out);
            collect_vars_stmts(b, out);
        }
        Stmt::Import { default, named, .. } => {
            if let Some(d) = default {
                out.remove(d);
            }
            for n in named {
                out.remove(n);
            }
        }
        Stmt::ExportDefault(e) => collect_vars_expr(e, out),
        Stmt::ExportNamed(names) => {
            for n in names {
                out.remove(n);
            }
        }
        Stmt::Switch(d, cases, def) => {
            collect_vars_expr(d, out);
            for case in cases {
                collect_vars_expr(&case.label, out);
                collect_vars_stmts(&case.body, out);
            }
            if let Some(body) = def {
                collect_vars_stmts(body, out);
            }
        }
        Stmt::TryCatch(try_b, catch, fin) => {
            collect_vars_stmts(try_b, out);
            if let Some((catch_var, catch_b)) = catch {
                out.remove(catch_var);
                collect_vars_stmts(catch_b, out);
            }
            if let Some(b) = fin {
                collect_vars_stmts(b, out);
            }
        }
        Stmt::While(c, b) => {
            collect_vars_expr(c, out);
            collect_vars_stmts(b, out);
        }
        Stmt::Break(_) | Stmt::Continue(_) => {}
        Stmt::Label(_, inner) => collect_vars_stmt(inner, out),
        Stmt::Block(stmts) => collect_vars_stmts(stmts, out),
        Stmt::DoWhile(body, cond) => {
            collect_vars_stmts(body, out);
            collect_vars_expr(cond, out);
        }
        Stmt::Throw(e) => collect_vars_expr(e, out),
        Stmt::Function(_, p, b) => {
            for (param, default) in p {
                out.remove(param);
                if let Some(e) = default {
                    collect_vars_expr(e, out);
                }
            }
            collect_vars_stmts(b, out);
        }
        Stmt::AsyncFunction(_, p, b) => {
            for (param, default) in p {
                out.remove(param);
                if let Some(e) = default {
                    collect_vars_expr(e, out);
                }
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
        LValue::This => {}
        LValue::Member(b, _) => collect_vars_lvalue(b, out),
        LValue::Index(b, i) => {
            collect_vars_lvalue(b, out);
            collect_vars_expr(i, out);
        }
        LValue::MemberExpr(b, _) => collect_vars_expr(b, out),
        LValue::IndexExpr(b, i) => {
            collect_vars_expr(b, out);
            collect_vars_expr(i, out);
        }
    }
}

fn collect_vars_expr(expr: &Expr, out: &mut HashSet<String>) {
    match expr {
        Expr::Var(n) => {
            if n != "document" && n != "console" {
                out.insert(n.clone());
            }
        }
        Expr::Member(b, _) | Expr::OptMember(b, _) => collect_vars_expr(b, out),
        Expr::Index(b, i) | Expr::OptIndex(b, i) => {
            collect_vars_expr(b, out);
            collect_vars_expr(i, out);
        }
        Expr::Call(c, args) | Expr::OptCall(c, args) => {
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
            for (param, default) in p {
                out.remove(param);
                if let Some(e) = default {
                    collect_vars_expr(e, out);
                }
            }
            collect_vars_expr(b, out);
        }
        Expr::Block(stmts) => collect_vars_stmts(stmts, out),
        Expr::Object(pairs) => {
            for (k, e) in pairs {
                match k {
                    ObjectEntryKey::Spread(expr) => collect_vars_expr(expr, out),
                    ObjectEntryKey::Computed(key_expr) => collect_vars_expr(key_expr, out),
                    ObjectEntryKey::Lit(_) => {}
                }
                collect_vars_expr(e, out);
            }
        }
        Expr::Array(elems) => {
            for e in elems {
                collect_vars_expr(e, out);
            }
        }
        Expr::New(_, a) => {
            for e in a {
                collect_vars_expr(e, out);
            }
        }
        Expr::Await(i) => collect_vars_expr(i, out),
        Expr::Seq(exprs) => {
            for e in exprs {
                collect_vars_expr(e, out);
            }
        }
        Expr::AssignExpr(lv, _, rhs) => {
            collect_vars_lvalue(lv, out);
            collect_vars_expr(rhs, out);
        }
        Expr::Cond(c, t, e) => {
            collect_vars_expr(c, out);
            collect_vars_expr(t, out);
            collect_vars_expr(e, out);
        }
        Expr::Update(lv, _, _) => collect_vars_lvalue(lv, out),
        Expr::FunExpr(p, body) => {
            for (param, default) in p {
                out.remove(param);
                if let Some(e) = default {
                    collect_vars_expr(e, out);
                }
            }
            collect_vars_stmts(body, out);
        }
        Expr::Template(parts) => {
            for part in parts {
                if let super::ast::TemplatePart::Expr(e) = part {
                    collect_vars_expr(e, out);
                }
            }
        }
        Expr::This | Expr::Lit(_) => {}
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
            if let Some(v) = inner.scope_get(name) {
                env.set(name.clone(), kv8_to_kabootar(&v));
            }
        }
        Ok(())
    })
}

pub fn sync_env_to_scope(ctx: &Kv8Context, names: &HashSet<String>, env: &Environment) -> Result<(), String> {
    ctx.with_mut(|inner| {
        for name in names {
            if let Some(v) = env.get(name) {
                inner.scope_current_mut().insert(name.clone(), super::eval::kabootar_to_kv8(v));
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
