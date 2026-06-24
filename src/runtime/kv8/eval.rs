//! Kv8 parser + interpreter — JS-subset with if/for/function/arrow + JIT hooks.

use super::ast::{Expr, Kv8Program, LValue, Stmt};
use super::bytecode_bridge::{compile_arrow, run_kv8_bytecode_fn};
use super::context::{Kv8Context, Kv8Value};
use super::jit::{loop_key, JIT_THRESHOLD};
use super::opt::{self, collect_vars_stmts, sync_env_to_scope, sync_scope_to_env};
use super::lexer::{tokenize, Token};
use crate::runtime::kabootar_dom::DomNode;
use std::collections::HashMap;

pub fn eval_script(ctx: &Kv8Context, source: &str) -> Result<Kv8Value, String> {
    let key = super::opt::hash_str(source);
    let program = ctx.with_mut(|inner| {
        if let Some(p) = inner.opt.program_cache.get(&key) {
            return Ok(p.clone());
        }
        let p = parse_program(source)?;
        inner.opt.program_cache.insert(key, p.clone());
        Ok(p)
    })?;
    run_program(ctx, &program)
}

pub fn parse_program(source: &str) -> Result<Kv8Program, String> {
    let tokens = tokenize(source)?;
    let mut p = Parser { tokens, pos: 0 };
    Ok(Kv8Program {
        stmts: p.parse_program()?,
    })
}

pub fn run_program(ctx: &Kv8Context, program: &Kv8Program) -> Result<Kv8Value, String> {
    let mut last = Kv8Value::Undefined;
    for stmt in &program.stmts {
        last = exec_stmt(ctx, stmt.clone())?;
    }
    ctx.with_mut(|inner| {
        inner.last_result = last.clone();
        Ok(())
    })?;
    Ok(last)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn bump(&mut self) -> Token {
        let t = self.peek().clone();
        if !matches!(t, Token::Eof) {
            self.pos += 1;
        }
        t
    }

    fn parse_program(&mut self) -> Result<Vec<Stmt>, String> {
        let mut out = Vec::new();
        while !matches!(self.peek(), Token::Eof) {
            out.push(self.parse_stmt()?);
        }
        Ok(out)
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.expect(Token::LBrace)?;
        let mut stmts = Vec::new();
        while !matches!(self.peek(), Token::RBrace | Token::Eof) {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(Token::RBrace)?;
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        match self.peek() {
            Token::Let | Token::Const => {
                self.bump();
                let Token::Ident(name) = self.bump() else {
                    return Err("expected name".into());
                };
                self.expect(Token::Eq)?;
                let expr = self.parse_expr()?;
                self.semi();
                Ok(Stmt::Let(name, expr))
            }
            Token::Return => {
                self.bump();
                let expr = self.parse_expr()?;
                self.semi();
                Ok(Stmt::Return(expr))
            }
            Token::If => {
                self.bump();
                self.expect(Token::LParen)?;
                let cond = self.parse_expr()?;
                self.expect(Token::RParen)?;
                let then_b = self.parse_block()?;
                let else_b = if matches!(self.peek(), Token::Else) {
                    self.bump();
                    Some(self.parse_block()?)
                } else {
                    None
                };
                Ok(Stmt::If(cond, then_b, else_b))
            }
            Token::For => {
                self.bump();
                self.expect(Token::LParen)?;
                self.expect(Token::Let)?;
                let Token::Ident(var) = self.bump() else {
                    return Err("for var".into());
                };
                self.expect(Token::Eq)?;
                let start = self.parse_expr()?;
                self.expect(Token::Semicolon)?;
                let cond = self.parse_expr()?;
                self.expect(Token::Semicolon)?;
                let Token::Ident(step_var) = self.bump() else {
                    return Err("for step var".into());
                };
                if step_var != var {
                    return Err("for step must use same variable".into());
                }
                self.expect(Token::Eq)?;
                let step = self.parse_expr()?;
                self.expect(Token::RParen)?;
                let body = self.parse_block()?;
                Ok(Stmt::For(var, start, cond, step, body))
            }
            Token::Function => {
                self.bump();
                let Token::Ident(name) = self.bump() else {
                    return Err("function name".into());
                };
                self.expect(Token::LParen)?;
                let params = self.parse_params()?;
                self.expect(Token::RParen)?;
                let body = self.parse_block()?;
                Ok(Stmt::Function(name, params, body))
            }
            _ => {
                if let Some(lv) = self.try_lvalue()? {
                    self.expect(Token::Eq)?;
                    let expr = self.parse_expr()?;
                    self.semi();
                    return Ok(Stmt::Assign(lv, expr));
                }
                let expr = self.parse_expr()?;
                self.semi();
                Ok(Stmt::Expr(expr))
            }
        }
    }

    fn parse_params(&mut self) -> Result<Vec<String>, String> {
        let mut params = Vec::new();
        if matches!(self.peek(), Token::RParen) {
            return Ok(params);
        }
        loop {
            let Token::Ident(n) = self.bump() else {
                return Err("param name".into());
            };
            params.push(n);
            if matches!(self.peek(), Token::Comma) {
                self.bump();
            } else {
                break;
            }
        }
        Ok(params)
    }

    fn try_lvalue(&mut self) -> Result<Option<LValue>, String> {
        let saved = self.pos;
        let Token::Ident(name) = self.peek().clone() else {
            return Ok(None);
        };
        self.bump();
        let mut lv = LValue::Name(name);
        while matches!(self.peek(), Token::Dot) {
            self.bump();
            let Token::Ident(field) = self.bump() else {
                self.pos = saved;
                return Ok(None);
            };
            lv = LValue::Member(Box::new(lv), field);
        }
        if matches!(self.peek(), Token::Eq) {
            Ok(Some(lv))
        } else {
            self.pos = saved;
            Ok(None)
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and()?;
        while matches!(self.peek(), Token::OrOr) {
            self.bump();
            left = Expr::Bin(Box::new(left), '|', Box::new(self.parse_and()?));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_equality()?;
        while matches!(self.peek(), Token::AndAnd) {
            self.bump();
            left = Expr::Bin(Box::new(left), '&', Box::new(self.parse_equality()?));
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_compare()?;
        while matches!(self.peek(), Token::EqEq | Token::Ne) {
            let op = match self.bump() {
                Token::EqEq => '=',
                Token::Ne => '!',
                _ => unreachable!(),
            };
            left = Expr::Bin(Box::new(left), op, Box::new(self.parse_compare()?));
        }
        Ok(left)
    }

    fn parse_compare(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_additive()?;
        while matches!(self.peek(), Token::Lt | Token::Le | Token::Gt | Token::Ge) {
            let op = match self.bump() {
                Token::Lt => '<',
                Token::Le => 'l',
                Token::Gt => '>',
                Token::Ge => 'g',
                _ => unreachable!(),
            };
            left = Expr::Bin(Box::new(left), op, Box::new(self.parse_additive()?));
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplicative()?;
        while matches!(self.peek(), Token::Plus | Token::Minus) {
            let op = match self.bump() {
                Token::Plus => '+',
                Token::Minus => '-',
                _ => unreachable!(),
            };
            left = Expr::Bin(Box::new(left), op, Box::new(self.parse_multiplicative()?));
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_primary()?;
        while matches!(self.peek(), Token::Star) {
            self.bump();
            left = Expr::Bin(Box::new(left), '*', Box::new(self.parse_primary()?));
        }
        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        if matches!(self.peek(), Token::LParen) {
            let saved = self.pos;
            self.bump();
            if let Ok(params) = self.parse_params() {
                if matches!(self.peek(), Token::RParen) {
                    self.bump();
                    if matches!(self.peek(), Token::FatArrow) {
                        self.bump();
                        let body = if matches!(self.peek(), Token::LBrace) {
                            let stmts = self.parse_block()?;
                            Expr::Block(stmts)
                        } else {
                            self.parse_expr()?
                        };
                        return Ok(Expr::Arrow(params, Box::new(body)));
                    }
                }
            }
            self.pos = saved;
        }
        let mut expr = match self.bump() {
            Token::Bang => Expr::Unary('!', Box::new(self.parse_primary()?)),
            Token::Number(n) => Expr::Lit(Kv8Value::Num(n)),
            Token::String(s) => Expr::Lit(Kv8Value::Str(s)),
            Token::True => Expr::Lit(Kv8Value::Bool(true)),
            Token::False => Expr::Lit(Kv8Value::Bool(false)),
            Token::Null => Expr::Lit(Kv8Value::Null),
            Token::Ident(name) => Expr::Var(name),
            Token::LParen => {
                let inner = self.parse_expr()?;
                if matches!(self.peek(), Token::FatArrow) {
                    self.bump();
                    let body = self.parse_expr()?;
                    self.expect(Token::RParen)?;
                    return Ok(Expr::Arrow(vec![], Box::new(body)));
                }
                self.expect(Token::RParen)?;
                inner
            }
            other => return Err(format!("unexpected expr token: {:?}", other)),
        };
        loop {
            match self.peek() {
                Token::Dot => {
                    self.bump();
                    let Token::Ident(field) = self.bump() else {
                        return Err("field".into());
                    };
                    expr = Expr::Member(Box::new(expr), field);
                }
                Token::LParen => {
                    self.bump();
                    let mut args = Vec::new();
                    if !matches!(self.peek(), Token::RParen) {
                        loop {
                            args.push(self.parse_expr()?);
                            if matches!(self.peek(), Token::Comma) {
                                self.bump();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(Token::RParen)?;
                    expr = Expr::Call(Box::new(expr), args);
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn expect(&mut self, want: Token) -> Result<(), String> {
        let got = self.bump();
        if std::mem::discriminant(&got) != std::mem::discriminant(&want) {
            return Err(format!("expected {:?}", want));
        }
        Ok(())
    }

    fn semi(&mut self) {
        if matches!(self.peek(), Token::Semicolon) {
            self.bump();
        }
    }
}

fn exec_stmt(ctx: &Kv8Context, stmt: Stmt) -> Result<Kv8Value, String> {
    match stmt {
        Stmt::Let(name, expr) => {
            let val = eval_expr(ctx, expr)?;
            ctx.with_mut(|inner| {
                inner.scope.insert(name, val.clone());
                Ok(val)
            })
        }
        Stmt::Assign(lv, expr) => {
            let val = eval_expr(ctx, expr)?;
            assign_lvalue(ctx, lv, val.clone())?;
            Ok(val)
        }
        Stmt::Return(expr) => eval_expr(ctx, expr),
        Stmt::Expr(expr) => eval_expr(ctx, expr),
        Stmt::If(cond, then_b, else_b) => {
            if eval_expr(ctx, cond)?.is_truthy() {
                run_stmts(ctx, &then_b)
            } else if let Some(e) = else_b {
                run_stmts(ctx, &e)
            } else {
                Ok(Kv8Value::Undefined)
            }
        }
        Stmt::For(var, start, cond, step, body) => {
            let start_v = eval_expr(ctx, start)?;
            ctx.with_mut(|inner| {
                inner.scope.insert(var.clone(), start_v);
                Ok(())
            })?;
            let key = loop_key(&var, &super::opt::expr_key(&cond));
            let mut last = Kv8Value::Undefined;
            let mut iter = 0u64;
            let mut body_refs = std::collections::HashSet::new();
            collect_vars_stmts(&body, &mut body_refs);
            body_refs.insert(var.clone());
            while eval_expr(ctx, cond.clone())?.is_truthy() {
                iter += 1;
                let jit_fn = ctx.with_mut(|inner| {
                    let jit = inner.jit.get_or_insert_with(Default::default);
                    if jit.record_loop(&key) {
                        let _ = jit.compile_loop(&key, &body);
                    }
                    Ok(jit.get_loop(&key).cloned())
                })?;
                if let Some(f) = jit_fn.filter(|_| iter > JIT_THRESHOLD) {
                    let mut env = crate::value::Environment::new();
                    sync_scope_to_env(ctx, &body_refs, &mut env)?;
                    let out = run_kv8_bytecode_fn(&f, vec![], &mut env)?;
                    sync_env_to_scope(ctx, &body_refs, &env)?;
                    last = kabootar_to_kv8(out);
                } else {
                    last = run_stmts(ctx, &body)?;
                }
                let step_v = eval_expr(ctx, step.clone())?;
                ctx.with_mut(|inner| {
                    inner.scope.insert(var.clone(), step_v);
                    Ok(())
                })?;
                if iter > 10_000 {
                    break;
                }
            }
            Ok(last)
        }
        Stmt::Function(name, params, body) => ctx.with_mut(|inner| {
            inner.scope.insert(
                name,
                Kv8Value::Fun {
                    params,
                    body,
                },
            );
            Ok(Kv8Value::Undefined)
        }),
    }
}

fn run_stmts(ctx: &Kv8Context, stmts: &[Stmt]) -> Result<Kv8Value, String> {
    let mut last = Kv8Value::Undefined;
    for s in stmts {
        last = exec_stmt(ctx, s.clone())?;
    }
    Ok(last)
}

fn assign_lvalue(ctx: &Kv8Context, lv: LValue, val: Kv8Value) -> Result<(), String> {
    match lv {
        LValue::Name(name) => ctx.with_mut(|inner| {
            inner.scope.insert(name, val);
            Ok(())
        }),
        LValue::Member(base, field) => {
            if let LValue::Member(ref el_lv, ref style_field) = *base {
                if style_field == "style" {
                    let el = eval_lvalue_as_obj(ctx, (**el_lv).clone())?;
                    if let Kv8Value::Dom(node) = &el {
                        if let Kv8Value::Str(s) = &val {
                            let _ = ctx.set_attr(node.id, &format!("style:{field}"), s);
                        }
                    }
                    if let LValue::Name(var) = &**el_lv {
                        if let Kv8Value::Dom(mut node) = el {
                            if let Kv8Value::Str(s) = val {
                                node.set_attr(&format!("style:{field}"), &s);
                                ctx.with_mut(|inner| {
                                    inner.scope.insert(var.clone(), Kv8Value::Dom(node));
                                    Ok(())
                                })?;
                            }
                            return Ok(());
                        }
                    }
                    return Ok(());
                }
            }
            let parent = eval_lvalue_as_obj(ctx, *base)?;
            match (&parent, field.as_str()) {
                (Kv8Value::Dom(node), "textContent") => {
                    if let Kv8Value::Str(s) = val {
                        ctx.set_text_content(node.id, &s)?;
                    }
                    Ok(())
                }
                _ => Err("cannot assign to member".into()),
            }
        }
    }
}

fn eval_lvalue_as_obj(ctx: &Kv8Context, lv: LValue) -> Result<Kv8Value, String> {
    match lv {
        LValue::Name(name) => ctx.with_mut(|inner| {
            Ok(inner.scope.get(&name).cloned().unwrap_or(Kv8Value::Undefined))
        }),
        LValue::Member(base, field) => {
            let v = eval_lvalue_as_obj(ctx, *base)?;
            member_get(ctx, v, &field)
        }
    }
}

fn eval_expr(ctx: &Kv8Context, expr: Expr) -> Result<Kv8Value, String> {
    match expr {
        Expr::Lit(v) => Ok(v),
        Expr::Var(name) => ctx.with_mut(|inner| {
            if name == "document" {
                if inner.opt.document_singleton.is_none() {
                    inner.opt.document_singleton = Some(document_object());
                }
                return Ok(inner.opt.document_singleton.clone().unwrap());
            }
            if name == "console" {
                if inner.opt.console_singleton.is_none() {
                    inner.opt.console_singleton = Some(console_object());
                }
                return Ok(inner.opt.console_singleton.clone().unwrap());
            }
            Ok(inner.scope.get(&name).cloned().unwrap_or(Kv8Value::Undefined))
        }),
        Expr::Member(base, field) => {
            let v = eval_expr(ctx, *base)?;
            member_get(ctx, v, &field)
        }
        Expr::Call(callee, args) => {
            let func = eval_expr(ctx, *callee)?;
            let evaluated: Result<Vec<Kv8Value>, String> =
                args.into_iter().map(|a| eval_expr(ctx, a)).collect();
            call_value(ctx, func, evaluated?)
        }
        Expr::Unary(op, inner) => {
            let v = eval_expr(ctx, *inner)?;
            match op {
                '!' => Ok(Kv8Value::Bool(!v.is_truthy())),
                _ => Err("unsupported unary".into()),
            }
        }
        Expr::Bin(l, op, r) => {
            let a = eval_expr(ctx, *l)?;
            let b = eval_expr(ctx, *r)?;
            Ok(eval_bin(op, &a, &b))
        }
        Expr::Arrow(params, body) => Ok(Kv8Value::Arrow {
            params,
            body,
        }),
        Expr::Block(stmts) => run_stmts(ctx, &stmts),
    }
}

pub(crate) fn kabootar_to_kv8(out: crate::value::Value) -> Kv8Value {
    match out {
        crate::value::Value::Number(n) => Kv8Value::Num(n as f64),
        crate::value::Value::String(s) => Kv8Value::Str(s),
        crate::value::Value::Bool(b) => Kv8Value::Bool(b),
        _ => Kv8Value::Null,
    }
}

fn eval_bin(op: char, a: &Kv8Value, b: &Kv8Value) -> Kv8Value {
    match op {
        '+' => match (a, b) {
            (Kv8Value::Str(x), Kv8Value::Str(y)) => Kv8Value::Str(format!("{x}{y}")),
            _ => Kv8Value::Num(a.as_num().unwrap_or(0.0) + b.as_num().unwrap_or(0.0)),
        },
        '-' => Kv8Value::Num(a.as_num().unwrap_or(0.0) - b.as_num().unwrap_or(0.0)),
        '*' => Kv8Value::Num(a.as_num().unwrap_or(0.0) * b.as_num().unwrap_or(0.0)),
        '=' => Kv8Value::Bool(a.as_num().unwrap_or(0.0) == b.as_num().unwrap_or(0.0)),
        '!' => Kv8Value::Bool(a.as_num().unwrap_or(0.0) != b.as_num().unwrap_or(0.0)),
        '<' => Kv8Value::Bool(a.as_num().unwrap_or(0.0) < b.as_num().unwrap_or(0.0)),
        'l' => Kv8Value::Bool(a.as_num().unwrap_or(0.0) <= b.as_num().unwrap_or(0.0)),
        '>' => Kv8Value::Bool(a.as_num().unwrap_or(0.0) > b.as_num().unwrap_or(0.0)),
        'g' => Kv8Value::Bool(a.as_num().unwrap_or(0.0) >= b.as_num().unwrap_or(0.0)),
        '&' => Kv8Value::Bool(a.is_truthy() && b.is_truthy()),
        '|' => Kv8Value::Bool(a.is_truthy() || b.is_truthy()),
        _ => Kv8Value::Undefined,
    }
}

fn document_object() -> Kv8Value {
    let mut doc = HashMap::new();
    doc.insert("__native".into(), Kv8Value::Str("document".into()));
    Kv8Value::Obj(doc)
}

fn console_object() -> Kv8Value {
    let mut c = HashMap::new();
    c.insert("__native".into(), Kv8Value::Str("console".into()));
    Kv8Value::Obj(c)
}

fn member_get(ctx: &Kv8Context, obj: Kv8Value, field: &str) -> Result<Kv8Value, String> {
    let base_tag = match &obj {
        Kv8Value::Obj(map) => map
            .get("__native")
            .and_then(|v| v.as_str())
            .unwrap_or("obj")
            .to_string(),
        Kv8Value::Dom(n) => format!("dom:{}", n.id),
        _ => "other".into(),
    };
    ctx.with_mut(|inner| {
        inner.opt.predictor.record_member(&base_tag, field);
        Ok(())
    })?;
    match obj {
        Kv8Value::Obj(map) => {
            if map.get("__native").and_then(|v| v.as_str()) == Some("document") {
                return Ok(document_method(field));
            }
            if map.get("__native").and_then(|v| v.as_str()) == Some("console") {
                return Ok(console_method(field));
            }
            Ok(map.get(field).cloned().unwrap_or(Kv8Value::Undefined))
        }
        Kv8Value::Dom(node) => match field {
            "id" => Ok(Kv8Value::Num(node.id as f64)),
            "tagName" => Ok(Kv8Value::Str(node.tag.clone())),
            "style" => Ok(style_object()),
            "textContent" => Ok(Kv8Value::Str(node.text.clone().unwrap_or_default())),
            "appendChild" => Ok(element_method("appendChild", node.id)),
            _ => Ok(Kv8Value::Undefined),
        },
        _ => Ok(Kv8Value::Undefined),
    }
}

fn style_object() -> Kv8Value {
    let mut style = HashMap::new();
    style.insert("__native".into(), Kv8Value::Str("style".into()));
    Kv8Value::Obj(style)
}

fn document_method(name: &str) -> Kv8Value {
    let mut m = HashMap::new();
    m.insert("__native".into(), Kv8Value::Str(format!("document.{name}")));
    Kv8Value::Obj(m)
}

fn console_method(name: &str) -> Kv8Value {
    let mut m = HashMap::new();
    m.insert("__native".into(), Kv8Value::Str(format!("console.{name}")));
    Kv8Value::Obj(m)
}

fn element_method(name: &str, id: u64) -> Kv8Value {
    let mut m = HashMap::new();
    m.insert("__native".into(), Kv8Value::Str(format!("element.{name}")));
    m.insert("__id".into(), Kv8Value::Num(id as f64));
    Kv8Value::Obj(m)
}

fn call_value(ctx: &Kv8Context, callee: Kv8Value, args: Vec<Kv8Value>) -> Result<Kv8Value, String> {
    match callee {
        Kv8Value::Fun { params, body } => {
            ctx.with_mut(|inner| {
                for (p, v) in params.iter().zip(args.iter()) {
                    inner.scope.insert(p.clone(), v.clone());
                }
                Ok(())
            })?;
            run_stmts(ctx, &body)
        }
        Kv8Value::Arrow { params, body } => match body.as_ref() {
            Expr::Block(stmts) => {
                ctx.with_mut(|inner| {
                    for (p, v) in params.iter().zip(args.iter()) {
                        inner.scope.insert(p.clone(), v.clone());
                    }
                    Ok(())
                })?;
                run_stmts(ctx, stmts)
            }
            expr => {
                let key = opt::arrow_cache_key(&params, expr);
                let compiled = ctx.with_mut(|inner| {
                    if let Some(c) = inner.opt.arrow_cache.get(&key) {
                        return Ok(c.clone());
                    }
                    let c = compile_arrow(&params, expr)?;
                    inner.opt.arrow_cache.insert(key, c.clone());
                    Ok(c)
                })?;
                let kabootar_args: Vec<crate::value::Value> = args
                    .iter()
                    .map(|v| opt::kv8_to_kabootar(v))
                    .collect();
                let mut env = crate::value::Environment::new();
                for (p, v) in params.iter().zip(kabootar_args.iter()) {
                    env.set(p.clone(), v.clone());
                }
                let out = run_kv8_bytecode_fn(&compiled, kabootar_args, &mut env)?;
                Ok(kabootar_to_kv8(out))
            }
        },
        Kv8Value::Obj(_) => call_native(ctx, callee, args),
        _ => Err("value is not callable".into()),
    }
}

fn call_native(ctx: &Kv8Context, callee: Kv8Value, args: Vec<Kv8Value>) -> Result<Kv8Value, String> {
    let Kv8Value::Obj(m) = callee else {
        return Err("not native".into());
    };
    let native = m.get("__native").and_then(|v| v.as_str()).map(str::to_string);
    let Some(native) = native else {
        return Err("value is not callable".into());
    };
    ctx.with_mut(|inner| {
        inner.opt.predictor.record_call(&native);
        Ok(())
    })?;
    match native.as_str() {
        "document.createElement" => {
            let tag = args.first().and_then(|v| v.as_str()).unwrap_or("div");
            Ok(Kv8Value::Dom(ctx.create_element(tag)?))
        }
        "document.querySelector" => {
            let sel = args.first().and_then(|v| v.as_str()).unwrap_or("div");
            match ctx.query_selector(sel)? {
                Some(n) => Ok(Kv8Value::Dom(n)),
                None => Ok(Kv8Value::Null),
            }
        }
        "document.appendChild" => {
            if let Some(Kv8Value::Dom(c)) = args.first() {
                let root_id = ctx.with_mut(|inner| Ok(inner.document.root.id))?;
                ctx.append_child(root_id, c.clone())?;
                Ok(Kv8Value::Dom(c.clone()))
            } else {
                Ok(Kv8Value::Null)
            }
        }
        "element.appendChild" => {
            let pid = m.get("__id").and_then(|v| v.as_num()).unwrap_or(0.0) as u64;
            if let Some(Kv8Value::Dom(child)) = args.first() {
                ctx.append_child(pid, child.clone())?;
            }
            Ok(Kv8Value::Null)
        }
        "console.log" => {
            let msgs: Vec<String> = args
                .iter()
                .map(|v| match v {
                    Kv8Value::Str(s) => s.clone(),
                    Kv8Value::Num(n) => n.to_string(),
                    Kv8Value::Bool(b) => b.to_string(),
                    _ => String::new(),
                })
                .collect();
            crate::runtime::browser_platform::kv8_console_log(&msgs);
            Ok(Kv8Value::Undefined)
        }
        _ => Err(format!("unknown native call: {native}")),
    }
}

pub fn dom_to_kabootar(node: &DomNode) -> crate::value::Value {
    crate::value::Value::KabootarDom(node.clone())
}
