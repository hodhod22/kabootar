//! Kv8 parser + interpreter — JS-subset with if/for/function/arrow + JIT hooks.

use super::ast::{Expr, Kv8Param, Kv8Program, LValue, ObjectEntryKey, Stmt, SwitchCase, TemplatePart};
use super::bytecode_bridge::{compile_arrow, run_kv8_bytecode_fn};
use super::context::{Kv8Context, Kv8ContextInner, Kv8Module, Kv8Value};
use super::jit::{loop_key, JIT_THRESHOLD};
use super::opt::{self, collect_vars_stmts, sync_env_to_scope, sync_scope_to_env, stmts_have_loop_control};
use super::lexer::{tokenize, TemplateSegment, Token};
use super::promise::{
    fulfill_promise, kv8_http_fetch, new_pending_promise, promise_rejected, promise_resolved,
    promise_state, push_then_link, reject_promise, Kv8PromiseState, Kv8ThenLink,
};
use crate::runtime::kabootar_dom::{assign_ids, DomNode};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
enum Flow {
    Next(Kv8Value),
    Return(Kv8Value),
    Break(Option<String>),
    Continue(Option<String>),
    Throw(Kv8Value),
}

fn flow_break_matches(label: &Option<String>, loop_label: Option<&str>) -> bool {
    match label {
        None => true,
        Some(l) => loop_label == Some(l.as_str()),
    }
}

fn flow_continue_matches(label: &Option<String>, loop_label: Option<&str>) -> bool {
    match label {
        None => true,
        Some(l) => loop_label == Some(l.as_str()),
    }
}

fn flow_fn_result(flow: Flow) -> Result<Kv8Value, String> {
    match flow {
        Flow::Return(v) | Flow::Next(v) => Ok(v),
        Flow::Break(_) => Err("illegal break".into()),
        Flow::Continue(_) => Err("illegal continue".into()),
        Flow::Throw(v) => Err(kv8_throw_string(&v)),
    }
}

fn flow_to_value(flow: Flow) -> Result<Kv8Value, String> {
    match flow {
        Flow::Next(v) => Ok(v),
        Flow::Return(v) => Ok(v),
        Flow::Break(_) => Err("illegal break".into()),
        Flow::Continue(_) => Err("illegal continue".into()),
        Flow::Throw(v) => Err(kv8_throw_string(&v)),
    }
}

fn kv8_throw_string(v: &Kv8Value) -> String {
    if let Kv8Value::Obj(m) = v {
        if let Some(Kv8Value::Str(msg)) = m.get("message") {
            return msg.clone();
        }
    }
    kv8_value_to_string(v)
}

fn current_this(ctx: &Kv8Context) -> Result<Kv8Value, String> {
    ctx.with_mut(|inner| {
        Ok(inner
            .this_stack
            .last()
            .cloned()
            .unwrap_or(Kv8Value::Undefined))
    })
}

fn push_this(ctx: &Kv8Context, value: Kv8Value) -> Result<(), String> {
    ctx.with_mut(|inner| {
        inner.this_stack.push(value);
        Ok(())
    })
}

fn pop_this(ctx: &Kv8Context) -> Result<(), String> {
    ctx.with_mut(|inner| {
        inner.this_stack.pop();
        Ok(())
    })
}

fn eval_call(ctx: &Kv8Context, callee: Expr, args: Vec<Expr>) -> Result<Kv8Value, String> {
    if let Expr::Member(base, field) = &callee {
        if field == "call" {
            let func = eval_expr(ctx, *base.clone())?;
            let evaled: Result<Vec<Kv8Value>, String> =
                args.into_iter().map(|a| eval_expr(ctx, a)).collect();
            let evaled = evaled?;
            let this_arg = evaled.first().cloned();
            let rest = evaled.into_iter().skip(1).collect();
            return call_value_with_this(ctx, func, rest, this_arg);
        }
        if field == "bind" {
            let func = eval_expr(ctx, *base.clone())?;
            let evaled: Result<Vec<Kv8Value>, String> =
                args.into_iter().map(|a| eval_expr(ctx, a)).collect();
            let evaled = evaled?;
            let this_arg = evaled
                .first()
                .cloned()
                .unwrap_or(Kv8Value::Undefined);
            let bound_args = evaled.into_iter().skip(1).collect();
            return Ok(make_bound_function(func, this_arg, bound_args));
        }
        if field == "apply" {
            let func = eval_expr(ctx, *base.clone())?;
            let evaled: Result<Vec<Kv8Value>, String> =
                args.into_iter().map(|a| eval_expr(ctx, a)).collect();
            let evaled = evaled?;
            let this_arg = evaled
                .first()
                .cloned()
                .unwrap_or(Kv8Value::Undefined);
            let call_args = evaled
                .get(1)
                .map(array_values_of)
                .unwrap_or_default();
            return call_value_with_this(ctx, func, call_args, Some(this_arg));
        }
    }
    let site = match &callee {
        Expr::Var(name) => name.clone(),
        Expr::Member(_, field) => format!(".{field}"),
        _ => "call".into(),
    };
    let (func, this_receiver) = match callee {
        Expr::Member(base, field) => {
            let receiver = eval_expr(ctx, *base)?;
            let func = member_get(ctx, receiver.clone(), &field)?;
            (func, Some(receiver))
        }
        other => (eval_expr(ctx, other)?, None),
    };
    if matches!(func, Kv8Value::Undefined) {
        return Err(format_call_error(
            ctx,
            format!("value is not callable: undefined at {site}"),
        ));
    }
    let _ = ctx.with_mut(|inner| {
        inner.call_trace.push(site);
        if inner.call_trace.len() > 48 {
            inner.call_trace.remove(0);
        }
        Ok(())
    });
    let evaluated: Result<Vec<Kv8Value>, String> =
        args.into_iter().map(|a| eval_expr(ctx, a)).collect();
    let result = call_value_with_this(ctx, func, evaluated?, this_receiver)
        .map_err(|e| format_call_error(ctx, e));
    let _ = ctx.with_mut(|inner| {
        inner.call_trace.pop();
        Ok(())
    });
    result
}

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
    ctx.reset_eval_ops();
    let mut last = Kv8Value::Undefined;
    for stmt in &program.stmts {
        match exec_stmt(ctx, stmt)? {
            Flow::Next(v) => last = v,
            Flow::Return(v) => {
                ctx.with_mut(|inner| {
                    inner.last_result = v.clone();
                    Ok(())
                })?;
                return Ok(v);
            }
            Flow::Break(_) => return Err("illegal break".into()),
            Flow::Continue(_) => return Err("illegal continue".into()),
            Flow::Throw(v) => return Err(kv8_throw_string(&v)),
        }
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
    fn err(&self, msg: impl Into<String>) -> String {
        let msg = msg.into();
        if let Some(tok) = self.tokens.get(self.pos) {
            format!("{msg} at token {pos} ({tok:?})", pos = self.pos)
        } else {
            msg
        }
    }
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
            out.extend(self.parse_stmt_list()?);
        }
        Ok(out)
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.expect(Token::LBrace)?;
        let mut stmts = Vec::new();
        while !matches!(self.peek(), Token::RBrace | Token::Eof) {
            stmts.extend(self.parse_stmt_list()?);
        }
        self.expect(Token::RBrace)?;
        Ok(stmts)
    }

    fn parse_block_or_stmt(&mut self) -> Result<Vec<Stmt>, String> {
        if matches!(self.peek(), Token::LBrace) {
            self.parse_block()
        } else {
            Ok(vec![self.parse_stmt()?])
        }
    }

    fn parse_stmt_list(&mut self) -> Result<Vec<Stmt>, String> {
        if matches!(self.peek(), Token::Var) {
            return self.parse_var_decls();
        }
        Ok(vec![self.parse_stmt()?])
    }

    fn parse_var_decls(&mut self) -> Result<Vec<Stmt>, String> {
        let out = self.parse_var_init_list()?;
        self.semi();
        Ok(out)
    }

    fn parse_var_init_list(&mut self) -> Result<Vec<Stmt>, String> {
        self.expect(Token::Var)?;
        let mut out = Vec::new();
        loop {
            let Token::Ident(name) = self.bump() else {
                return Err("expected var name".into());
            };
            let expr = if matches!(self.peek(), Token::Eq) {
                self.bump();
                self.parse_assign()?
            } else {
                Expr::Lit(Kv8Value::Undefined)
            };
            out.push(Stmt::Var(name, expr));
            if matches!(self.peek(), Token::Comma) {
                self.bump();
            } else {
                break;
            }
        }
        Ok(out)
    }

    fn parse_case_body(&mut self) -> Result<Vec<Stmt>, String> {
        let mut out = Vec::new();
        while !matches!(
            self.peek(),
            Token::Case | Token::Default | Token::RBrace | Token::Eof
        ) {
            out.extend(self.parse_stmt_list()?);
        }
        Ok(out)
    }

    fn is_ident(peek: &Token, name: &str) -> bool {
        matches!(peek, Token::Ident(s) if s == name)
    }

    fn parse_for_loop(&mut self) -> Result<Stmt, String> {
        self.bump();
        self.expect(Token::LParen)?;

        if matches!(self.peek(), Token::Var) {
            let saved = self.pos;
            self.bump();
            let Token::Ident(var) = self.bump() else {
                return Err("for var name".into());
            };
            if Self::is_ident(self.peek(), "of") {
                self.bump();
                let iterable = self.parse_expr()?;
                self.expect(Token::RParen)?;
                let body = self.parse_block_or_stmt()?;
                return Ok(Stmt::ForOf(LValue::Name(var), iterable, body));
            }
            if matches!(self.peek(), Token::In) {
                self.bump();
                let iterable = self.parse_expr()?;
                self.expect(Token::RParen)?;
                let body = self.parse_block_or_stmt()?;
                return Ok(Stmt::ForIn(LValue::Name(var), iterable, body));
            }
            self.pos = saved;
        }

        if matches!(self.peek(), Token::Let | Token::Const) {
            let saved = self.pos;
            self.bump();
            let Token::Ident(var) = self.bump() else {
                return Err("for let/const name".into());
            };
            if Self::is_ident(self.peek(), "of") {
                self.bump();
                let iterable = self.parse_expr()?;
                self.expect(Token::RParen)?;
                let body = self.parse_block_or_stmt()?;
                return Ok(Stmt::ForOf(LValue::Name(var), iterable, body));
            }
            self.pos = saved;
        }

        let saved = self.pos;
        if let Some(lv) = self.parse_lvalue_chain()? {
            if Self::is_ident(self.peek(), "of") {
                self.bump();
                let iterable = self.parse_expr()?;
                self.expect(Token::RParen)?;
                let body = self.parse_block_or_stmt()?;
                return Ok(Stmt::ForOf(lv, iterable, body));
            }
            if matches!(self.peek(), Token::In) {
                self.bump();
                let iterable = self.parse_expr()?;
                self.expect(Token::RParen)?;
                let body = self.parse_block_or_stmt()?;
                return Ok(Stmt::ForIn(lv, iterable, body));
            }
        }
        self.pos = saved;

        if matches!(self.peek(), Token::Let) {
            self.bump();
            let Token::Ident(var) = self.bump() else {
                return Err("for var".into());
            };
            self.expect(Token::Eq)?;
            let start = self.parse_assign()?;
            self.expect(Token::Semicolon)?;
            let cond = self.parse_assign()?;
            self.expect(Token::Semicolon)?;
            let Token::Ident(step_var) = self.bump() else {
                return Err("for step var".into());
            };
            if step_var != var {
                return Err("for step must use same variable".into());
            }
            self.expect(Token::Eq)?;
            let step = self.parse_assign()?;
            self.expect(Token::RParen)?;
            let body = self.parse_block_or_stmt()?;
            return Ok(Stmt::For(var, start, cond, step, body));
        }

        self.pos = saved;
        let init = self.parse_for_init()?;
        self.expect(Token::Semicolon)?;
        let cond = if matches!(self.peek(), Token::Semicolon) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.expect(Token::Semicolon)?;
        let update = if matches!(self.peek(), Token::RParen) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.expect(Token::RParen)?;
        let body = self.parse_block_or_stmt()?;
        Ok(Stmt::ForClassic(init, cond, update, body))
    }

    fn parse_for_init(&mut self) -> Result<Vec<Stmt>, String> {
        if matches!(self.peek(), Token::Semicolon) {
            return Ok(Vec::new());
        }
        if matches!(self.peek(), Token::Var) {
            return self.parse_var_init_list();
        }
        Ok(vec![Stmt::Expr(self.parse_expr()?)])
    }

    fn parse_function_expr(&mut self) -> Result<Expr, String> {
        self.expect(Token::Function)?;
        self.parse_function_expr_body()
    }

    fn parse_function_expr_body(&mut self) -> Result<Expr, String> {
        if let Token::Ident(_) = self.peek() {
            let saved = self.pos;
            self.bump();
            if !matches!(self.peek(), Token::LParen) {
                self.pos = saved;
            }
        }
        self.expect(Token::LParen)?;
        let params = self.parse_params()?;
        self.expect(Token::RParen)?;
        let body = self.parse_block()?;
        Ok(Expr::FunExpr(params, body))
    }

    fn parse_member_field(&mut self) -> Result<String, String> {
        match self.bump() {
            Token::Ident(name) => Ok(name),
            Token::For => Ok("for".into()),
            Token::While => Ok("while".into()),
            Token::Break => Ok("break".into()),
            Token::Continue => Ok("continue".into()),
            Token::If => Ok("if".into()),
            Token::Else => Ok("else".into()),
            Token::Return => Ok("return".into()),
            Token::New => Ok("new".into()),
            Token::This => Ok("this".into()),
            Token::Switch => Ok("switch".into()),
            Token::Case => Ok("case".into()),
            Token::Default => Ok("default".into()),
            Token::Catch => Ok("catch".into()),
            Token::In => Ok("in".into()),
            Token::Async => Ok("async".into()),
            Token::Await => Ok("await".into()),
            Token::Delete => Ok("delete".into()),
            Token::Instanceof => Ok("instanceof".into()),
            Token::Typeof => Ok("typeof".into()),
            Token::Void => Ok("void".into()),
            Token::Throw => Ok("throw".into()),
            Token::Try => Ok("try".into()),
            Token::Finally => Ok("finally".into()),
            Token::Do => Ok("do".into()),
            Token::Function => Ok("function".into()),
            Token::Var => Ok("var".into()),
            Token::Let => Ok("let".into()),
            Token::Const => Ok("const".into()),
            other => Err(format!("expected member field, got {:?}", other)),
        }
    }

    fn parse_import(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Import)?;
        let mut default = None;
        let mut named = Vec::new();
        if matches!(self.peek(), Token::LBrace) {
            self.bump();
            loop {
                let Token::Ident(name) = self.bump() else {
                    return Err("expected import name".into());
                };
                named.push(name);
                if matches!(self.peek(), Token::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }
            self.expect(Token::RBrace)?;
        } else {
            let Token::Ident(name) = self.bump() else {
                return Err("expected import binding".into());
            };
            if matches!(self.peek(), Token::Comma) {
                default = Some(name);
                self.bump();
                self.expect(Token::LBrace)?;
                loop {
                    let Token::Ident(n) = self.bump() else {
                        return Err("expected import name".into());
                    };
                    named.push(n);
                    if matches!(self.peek(), Token::Comma) {
                        self.bump();
                    } else {
                        break;
                    }
                }
                self.expect(Token::RBrace)?;
            } else {
                default = Some(name);
            }
        }
        self.expect(Token::From)?;
        let from = match self.bump() {
            Token::String(s) => s,
            other => return Err(self.err(format!("expected module string, got {other:?}"))),
        };
        self.semi();
        Ok(Stmt::Import {
            default,
            named,
            from,
        })
    }

    fn parse_export(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Export)?;
        if matches!(self.peek(), Token::Default) {
            self.bump();
            let expr = self.parse_assign()?;
            self.semi();
            return Ok(Stmt::ExportDefault(expr));
        }
        if matches!(self.peek(), Token::LBrace) {
            self.bump();
            let mut names = Vec::new();
            loop {
                let Token::Ident(name) = self.bump() else {
                    return Err("expected export name".into());
                };
                names.push(name);
                if matches!(self.peek(), Token::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }
            self.expect(Token::RBrace)?;
            self.semi();
            return Ok(Stmt::ExportNamed(names));
        }
        Err(self.err("unsupported export form"))
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        if let Token::Ident(name) = self.peek().clone() {
            if matches!(self.tokens.get(self.pos + 1), Some(Token::Colon)) {
                self.bump();
                self.expect(Token::Colon)?;
                let inner = if matches!(self.peek(), Token::LBrace) {
                    Stmt::Block(self.parse_block()?)
                } else {
                    self.parse_stmt()?
                };
                return Ok(Stmt::Label(name, Box::new(inner)));
            }
        }
        match self.peek() {
            Token::Semicolon => {
                self.bump();
                Ok(Stmt::Expr(Expr::Lit(Kv8Value::Undefined)))
            }
            Token::Var => {
                let stmts = self.parse_var_decls()?;
                Ok(if stmts.len() == 1 {
                    stmts.into_iter().next().unwrap()
                } else {
                    Stmt::Block(stmts)
                })
            }
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
                let expr = if matches!(
                    self.peek(),
                    Token::Semicolon | Token::RBrace | Token::Eof
                ) {
                    Expr::Lit(Kv8Value::Undefined)
                } else {
                    self.parse_expr()?
                };
                self.semi();
                Ok(Stmt::Return(expr))
            }
            Token::If => {
                self.bump();
                self.expect(Token::LParen)?;
                let cond = self.parse_expr()?;
                self.expect(Token::RParen)?;
                let then_b = self.parse_block_or_stmt()?;
                let else_b = if matches!(self.peek(), Token::Else) {
                    self.bump();
                    Some(self.parse_block_or_stmt()?)
                } else {
                    None
                };
                Ok(Stmt::If(cond, then_b, else_b))
            }
            Token::For => self.parse_for_loop(),
            Token::Switch => {
                self.bump();
                self.expect(Token::LParen)?;
                let disc = self.parse_expr()?;
                self.expect(Token::RParen)?;
                let mut cases = Vec::new();
                let mut default_body = None;
                self.expect(Token::LBrace)?;
                while !matches!(self.peek(), Token::RBrace | Token::Eof) {
                    if matches!(self.peek(), Token::Case) {
                        self.bump();
                        let label = self.parse_assign()?;
                        self.expect(Token::Colon)?;
                        let body = self.parse_case_body()?;
                        cases.push(SwitchCase { label, body });
                    } else if matches!(self.peek(), Token::Default) {
                        self.bump();
                        self.expect(Token::Colon)?;
                        default_body = Some(self.parse_case_body()?);
                    } else {
                        return Err("expected case or default in switch".into());
                    }
                }
                self.expect(Token::RBrace)?;
                Ok(Stmt::Switch(disc, cases, default_body))
            }
            Token::Try => {
                self.bump();
                let try_body = self.parse_block()?;
                let catch_clause = if matches!(self.peek(), Token::Catch) {
                    self.bump();
                    self.expect(Token::LParen)?;
                    let Token::Ident(catch_var) = self.bump() else {
                        return Err("catch variable name".into());
                    };
                    self.expect(Token::RParen)?;
                    let catch_body = self.parse_block()?;
                    Some((catch_var, catch_body))
                } else {
                    None
                };
                let finally_body = if matches!(self.peek(), Token::Finally) {
                    self.bump();
                    Some(self.parse_block()?)
                } else {
                    None
                };
                if catch_clause.is_none() && finally_body.is_none() {
                    return Err("try must have catch or finally".into());
                }
                Ok(Stmt::TryCatch(try_body, catch_clause, finally_body))
            }
            Token::While => {
                self.bump();
                self.expect(Token::LParen)?;
                let cond = self.parse_expr()?;
                self.expect(Token::RParen)?;
                let body = self.parse_block_or_stmt()?;
                Ok(Stmt::While(cond, body))
            }
            Token::Do => {
                self.bump();
                let body = self.parse_block_or_stmt()?;
                self.expect(Token::While)?;
                self.expect(Token::LParen)?;
                let cond = self.parse_expr()?;
                self.expect(Token::RParen)?;
                self.semi();
                Ok(Stmt::DoWhile(body, cond))
            }
            Token::Break => {
                self.bump();
                let label = if let Token::Ident(l) = self.peek().clone() {
                    self.bump();
                    Some(l)
                } else {
                    None
                };
                self.semi();
                Ok(Stmt::Break(label))
            }
            Token::Continue => {
                self.bump();
                let label = if let Token::Ident(l) = self.peek().clone() {
                    self.bump();
                    Some(l)
                } else {
                    None
                };
                self.semi();
                Ok(Stmt::Continue(label))
            }
            Token::Import => self.parse_import(),
            Token::Export => self.parse_export(),
            Token::Throw => {
                self.bump();
                let expr = self.parse_expr()?;
                self.semi();
                Ok(Stmt::Throw(expr))
            }
            Token::Function => {
                self.bump();
                let Token::Ident(name) = self.bump() else {
                    return Err("function name".into());
                };
                self.expect(Token::LParen)?;
                let params = self.parse_params()?;
                self.expect(Token::RParen)?;
                let body = self.parse_block_or_stmt()?;
                Ok(Stmt::Function(name, params, body))
            }
            Token::Async => {
                self.bump();
                self.expect(Token::Function)?;
                let Token::Ident(name) = self.bump() else {
                    return Err("async function name".into());
                };
                self.expect(Token::LParen)?;
                let params = self.parse_params()?;
                self.expect(Token::RParen)?;
                let body = self.parse_block_or_stmt()?;
                Ok(Stmt::AsyncFunction(name, params, body))
            }
            _ => {
                let expr = self.parse_expr()?;
                self.semi();
                Ok(Stmt::Expr(expr))
            }
        }
    }

    fn parse_params(&mut self) -> Result<Vec<Kv8Param>, String> {
        let mut params = Vec::new();
        if matches!(self.peek(), Token::RParen) {
            return Ok(params);
        }
        loop {
            let Token::Ident(n) = self.bump() else {
                return Err("param name".into());
            };
            let default = if matches!(self.peek(), Token::Eq) {
                self.bump();
                Some(self.parse_assign()?)
            } else {
                None
            };
            params.push((n, default));
            if matches!(self.peek(), Token::Comma) {
                self.bump();
            } else {
                break;
            }
        }
        Ok(params)
    }

    fn finish_lvalue(&mut self, mut lv: LValue) -> Result<LValue, String> {
        loop {
            match self.peek() {
                Token::Dot => {
                    self.bump();
                    let field = self.parse_member_field()?;
                    lv = LValue::Member(Box::new(lv), field);
                }
                Token::LBracket => {
                    self.bump();
                    let idx = self.parse_assign()?;
                    self.expect(Token::RBracket)?;
                    lv = LValue::Index(Box::new(lv), Box::new(idx));
                }
                _ => break,
            }
        }
        Ok(lv)
    }

    fn parse_lvalue_chain(&mut self) -> Result<Option<LValue>, String> {
        let saved = self.pos;
        let base = if matches!(self.peek(), Token::This) {
            self.bump();
            LValue::This
        } else if let Token::Ident(name) = self.peek().clone() {
            self.bump();
            LValue::Name(name)
        } else {
            return Ok(None);
        };
        if matches!(self.peek(), Token::LParen) {
            if !matches!(base, LValue::This) {
                self.pos = saved;
                return Ok(None);
            }
        }
        Ok(Some(self.finish_lvalue(base)?))
    }

    fn try_lvalue(&mut self) -> Result<Option<LValue>, String> {
        let saved = self.pos;
        let Some(lv) = self.parse_lvalue_chain()? else {
            return Ok(None);
        };
        if matches!(self.peek(), Token::Eq) {
            return Ok(Some(lv));
        }
        self.pos = saved;
        Ok(None)
    }

    fn try_assign_target(&mut self) -> Result<Option<(LValue, char)>, String> {
        let saved = self.pos;
        let Some(lv) = self.parse_lvalue_chain()? else {
            return Ok(None);
        };
        let op = match self.peek() {
            Token::Eq => '=',
            Token::PlusEq => '+',
            Token::MinusEq => '-',
            Token::StarEq => '*',
            Token::SlashEq => '/',
            Token::AmpEq => 'A',
            Token::PipeEq => 'o',
            Token::ShlEq => 'L',
            Token::ShrEq => 'R',
            Token::UshrEq => 'U',
            _ => {
                self.pos = saved;
                return Ok(None);
            }
        };
        self.bump();
        Ok(Some((lv, op)))
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_comma()
    }

    fn parse_comma(&mut self) -> Result<Expr, String> {
        let first = self.parse_assign()?;
        if !matches!(self.peek(), Token::Comma) {
            return Ok(first);
        }
        let mut seq = vec![first];
        while matches!(self.peek(), Token::Comma) {
            self.bump();
            seq.push(self.parse_assign()?);
        }
        Ok(Expr::Seq(seq))
    }

    fn parse_assign(&mut self) -> Result<Expr, String> {
        self.parse_assign_rhs()
    }

    /// Assignment and RHS chain — comma is handled only at `parse_comma` level.
    fn parse_assign_rhs(&mut self) -> Result<Expr, String> {
        let saved = self.pos;
        if let Some((lv, op)) = self.try_assign_target()? {
            let rhs = self.parse_assign_rhs()?;
            return Ok(Expr::AssignExpr(lv, op, Box::new(rhs)));
        }
        self.pos = saved;
        let expr = self.parse_cond()?;
        if let Some(lv) = expr_to_lvalue(&expr) {
            let op = match self.peek() {
                Token::Eq => {
                    self.bump();
                    '='
                }
                Token::PlusEq => {
                    self.bump();
                    '+'
                }
                Token::MinusEq => {
                    self.bump();
                    '-'
                }
                Token::StarEq => {
                    self.bump();
                    '*'
                }
                Token::SlashEq => {
                    self.bump();
                    '/'
                }
                Token::AmpEq => {
                    self.bump();
                    'A'
                }
                Token::PipeEq => {
                    self.bump();
                    'o'
                }
                Token::ShlEq => {
                    self.bump();
                    'L'
                }
                Token::ShrEq => {
                    self.bump();
                    'R'
                }
                Token::UshrEq => {
                    self.bump();
                    'U'
                }
                _ => return Ok(expr),
            };
            let rhs = self.parse_assign_rhs()?;
            return Ok(Expr::AssignExpr(lv, op, Box::new(rhs)));
        }
        Ok(expr)
    }

    fn parse_cond(&mut self) -> Result<Expr, String> {
        let cond = self.parse_nullish()?;
        if !matches!(self.peek(), Token::Question) {
            return Ok(cond);
        }
        self.bump();
        let then_e = self.parse_assign()?;
        self.expect(Token::Colon)?;
        let else_e = self.parse_assign()?;
        Ok(Expr::Cond(Box::new(cond), Box::new(then_e), Box::new(else_e)))
    }

    fn parse_nullish(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_or()?;
        while matches!(self.peek(), Token::NullishCoalesce) {
            self.bump();
            left = Expr::Bin(Box::new(left), 'n', Box::new(self.parse_or()?));
        }
        Ok(left)
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
        let mut left = self.parse_bitor()?;
        while matches!(self.peek(), Token::AndAnd) {
            self.bump();
            left = Expr::Bin(Box::new(left), '&', Box::new(self.parse_bitor()?));
        }
        Ok(left)
    }

    fn parse_bitor(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitxor()?;
        while matches!(self.peek(), Token::Pipe) {
            self.bump();
            left = Expr::Bin(Box::new(left), 'o', Box::new(self.parse_bitxor()?));
        }
        Ok(left)
    }

    fn parse_bitxor(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_bitand()?;
        while matches!(self.peek(), Token::Caret) {
            self.bump();
            left = Expr::Bin(Box::new(left), '^', Box::new(self.parse_bitand()?));
        }
        Ok(left)
    }

    fn parse_bitand(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_equality()?;
        while matches!(self.peek(), Token::Amp) {
            self.bump();
            left = Expr::Bin(Box::new(left), 'A', Box::new(self.parse_equality()?));
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_compare()?;
        while matches!(
            self.peek(),
            Token::EqEq | Token::Ne | Token::StrictEq | Token::StrictNe
        ) {
            let op = match self.bump() {
                Token::EqEq => '=',
                Token::Ne => '!',
                Token::StrictEq => 'E',
                Token::StrictNe => 'e',
                _ => unreachable!(),
            };
            left = Expr::Bin(Box::new(left), op, Box::new(self.parse_compare()?));
        }
        Ok(left)
    }

    fn parse_compare(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_shift()?;
        while matches!(
            self.peek(),
            Token::Lt | Token::Le | Token::Gt | Token::Ge | Token::In | Token::Instanceof
        ) {
            let op = match self.bump() {
                Token::Lt => '<',
                Token::Le => 'l',
                Token::Gt => '>',
                Token::Ge => 'g',
                Token::In => 'i',
                Token::Instanceof => 'I',
                _ => unreachable!(),
            };
            left = Expr::Bin(Box::new(left), op, Box::new(self.parse_shift()?));
        }
        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_additive()?;
        while matches!(self.peek(), Token::Shl | Token::Shr | Token::Ushr) {
            let op = match self.bump() {
                Token::Shl => 'L',
                Token::Shr => 'R',
                Token::Ushr => 'U',
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
        let mut left = self.parse_unary()?;
        while matches!(self.peek(), Token::Star | Token::Slash | Token::Percent) {
            let op = match self.bump() {
                Token::Star => '*',
                Token::Slash => '/',
                Token::Percent => '%',
                _ => unreachable!(),
            };
            left = Expr::Bin(Box::new(left), op, Box::new(self.parse_unary()?));
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Token::Await => {
                self.bump();
                Ok(Expr::Await(Box::new(self.parse_unary()?)))
            }
            Token::New => {
                self.bump();
                let callee = self.parse_new_target()?;
                let mut args = Vec::new();
                if matches!(self.peek(), Token::LParen) {
                    self.bump();
                    if !matches!(self.peek(), Token::RParen) {
                        loop {
                            args.push(self.parse_assign()?);
                            if matches!(self.peek(), Token::Comma) {
                                self.bump();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(Token::RParen)?;
                }
                Ok(Expr::New(Box::new(callee), args))
            }
            Token::Bang => {
                self.bump();
                Ok(Expr::Unary('!', Box::new(self.parse_unary()?)))
            }
            Token::Tilde => {
                self.bump();
                Ok(Expr::Unary('~', Box::new(self.parse_unary()?)))
            }
            Token::Minus => {
                self.bump();
                Ok(Expr::Unary('-', Box::new(self.parse_unary()?)))
            }
            Token::Plus => {
                self.bump();
                Ok(Expr::Unary('+', Box::new(self.parse_unary()?)))
            }
            Token::Typeof => {
                self.bump();
                Ok(Expr::Unary('t', Box::new(self.parse_unary()?)))
            }
            Token::Void => {
                self.bump();
                Ok(Expr::Unary('v', Box::new(self.parse_unary()?)))
            }
            Token::Delete => {
                self.bump();
                Ok(Expr::Unary('d', Box::new(self.parse_unary()?)))
            }
            Token::PlusPlus => {
                self.bump();
                let inner = self.parse_unary()?;
                let lv = expr_to_lvalue(&inner).ok_or("prefix ++ needs lvalue")?;
                Ok(Expr::Update(lv, '+', true))
            }
            Token::MinusMinus => {
                self.bump();
                let inner = self.parse_unary()?;
                let lv = expr_to_lvalue(&inner).ok_or("prefix -- needs lvalue")?;
                Ok(Expr::Update(lv, '-', true))
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_new_target(&mut self) -> Result<Expr, String> {
        if matches!(self.peek(), Token::Function) {
            return self.parse_function_expr();
        }
        let mut expr = self.parse_primary_base()?;
        loop {
            match self.peek() {
                Token::Dot => {
                    self.bump();
                    let field = self.parse_member_field()?;
                    expr = Expr::Member(Box::new(expr), field);
                }
                Token::OptionalDot => {
                    self.bump();
                    match self.peek() {
                        Token::LBracket => {
                            self.bump();
                            let idx = self.parse_assign()?;
                            self.expect(Token::RBracket)?;
                            expr = Expr::OptIndex(Box::new(expr), Box::new(idx));
                        }
                        Token::LParen => {
                            self.bump();
                            let mut args = Vec::new();
                            if !matches!(self.peek(), Token::RParen) {
                                loop {
                                    args.push(self.parse_assign()?);
                                    if matches!(self.peek(), Token::Comma) {
                                        self.bump();
                                    } else {
                                        break;
                                    }
                                }
                            }
                            self.expect(Token::RParen)?;
                            expr = Expr::OptCall(Box::new(expr), args);
                        }
                        _ => {
                            let field = self.parse_member_field()?;
                            expr = Expr::OptMember(Box::new(expr), field);
                        }
                    }
                }
                Token::LBracket => {
                    self.bump();
                    let idx = self.parse_assign()?;
                    self.expect(Token::RBracket)?;
                    expr = Expr::Index(Box::new(expr), Box::new(idx));
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary_base()?;
        self.finish_primary_postfix(&mut expr, true)?;
        if let Some(lv) = expr_to_lvalue(&expr) {
            if matches!(self.peek(), Token::PlusPlus) {
                self.bump();
                return Ok(Expr::Update(lv, '+', false));
            }
            if matches!(self.peek(), Token::MinusMinus) {
                self.bump();
                return Ok(Expr::Update(lv, '-', false));
            }
        }
        Ok(expr)
    }

    fn parse_primary_base(&mut self) -> Result<Expr, String> {
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
        match self.bump() {
            Token::Bang => Ok(Expr::Unary('!', Box::new(self.parse_primary()?))),
            Token::Number(n) => Ok(Expr::Lit(Kv8Value::Num(n))),
            Token::String(s) => Ok(Expr::Lit(Kv8Value::Str(s))),
            Token::True => Ok(Expr::Lit(Kv8Value::Bool(true))),
            Token::False => Ok(Expr::Lit(Kv8Value::Bool(false))),
            Token::Null => Ok(Expr::Lit(Kv8Value::Null)),
            Token::Undefined => Ok(Expr::Lit(Kv8Value::Undefined)),
            Token::This => Ok(Expr::This),
            Token::Function => self.parse_function_expr_body(),
            Token::Ident(name) => Ok(Expr::Var(name)),
            Token::LParen => {
                let inner = self.parse_expr()?;
                if matches!(self.peek(), Token::FatArrow) {
                    self.bump();
                    let body = self.parse_expr()?;
                    self.expect(Token::RParen)?;
                    return Ok(Expr::Arrow(vec![], Box::new(body)));
                }
                self.expect(Token::RParen)?;
                Ok(inner)
            }
            Token::LBrace => self.parse_object_literal(),
            Token::LBracket => self.parse_array_literal(),
            Token::Regex(pattern, flags) => Ok(Expr::Lit(regex_object(&pattern, &flags))),
            Token::Template(segments) => self.parse_template_expr(segments),
            other => Err(self.err(format!("unexpected expr token: {:?}", other))),
        }
    }

    fn parse_template_expr(&mut self, segments: Vec<TemplateSegment>) -> Result<Expr, String> {
        let mut parts = Vec::new();
        for seg in segments {
            match seg {
                TemplateSegment::Lit(s) => parts.push(TemplatePart::Lit(s)),
                TemplateSegment::Expr(src) => {
                    let tokens = tokenize(&src)?;
                    let mut sub = Parser { tokens, pos: 0 };
                    let e = sub.parse_assign()?;
                    if !matches!(sub.peek(), Token::Eof) {
                        return Err(sub.err("extra tokens in template expression"));
                    }
                    parts.push(TemplatePart::Expr(Box::new(e)));
                }
            }
        }
        Ok(Expr::Template(parts))
    }

    fn finish_primary_postfix(&mut self, expr: &mut Expr, allow_call: bool) -> Result<(), String> {
        loop {
            match self.peek() {
                Token::Dot => {
                    self.bump();
                    let field = self.parse_member_field()?;
                    *expr = Expr::Member(Box::new(expr.clone()), field);
                }
                Token::OptionalDot => {
                    self.bump();
                    match self.peek() {
                        Token::LBracket => {
                            self.bump();
                            let idx = self.parse_assign()?;
                            self.expect(Token::RBracket)?;
                            *expr = Expr::OptIndex(Box::new(expr.clone()), Box::new(idx));
                        }
                        Token::LParen => {
                            self.bump();
                            let mut args = Vec::new();
                            if !matches!(self.peek(), Token::RParen) {
                                loop {
                                    args.push(self.parse_assign()?);
                                    if matches!(self.peek(), Token::Comma) {
                                        self.bump();
                                    } else {
                                        break;
                                    }
                                }
                            }
                            self.expect(Token::RParen)?;
                            *expr = Expr::OptCall(Box::new(expr.clone()), args);
                        }
                        _ => {
                            let field = self.parse_member_field()?;
                            *expr = Expr::OptMember(Box::new(expr.clone()), field);
                        }
                    }
                }
                Token::LBracket => {
                    self.bump();
                    let idx = self.parse_assign()?;
                    self.expect(Token::RBracket)?;
                    *expr = Expr::Index(Box::new(expr.clone()), Box::new(idx));
                }
                Token::LParen if allow_call => {
                    if matches!(expr, Expr::Object(_)) {
                        let next = self.tokens.get(self.pos + 1);
                        if !matches!(next, Some(Token::RParen)) {
                            break;
                        }
                    }
                    self.bump();
                    let mut args = Vec::new();
                    if !matches!(self.peek(), Token::RParen) {
                        loop {
                            args.push(self.parse_assign()?);
                            if matches!(self.peek(), Token::Comma) {
                                self.bump();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(Token::RParen)?;
                    *expr = Expr::Call(Box::new(expr.clone()), args);
                }
                _ => break,
            }
        }
        Ok(())
    }

    fn expect(&mut self, want: Token) -> Result<(), String> {
        let got = self.bump();
        if std::mem::discriminant(&got) != std::mem::discriminant(&want) {
            return Err(self.err(format!("expected {want:?}, got {got:?}")));
        }
        Ok(())
    }

    fn semi(&mut self) {
        if matches!(self.peek(), Token::Semicolon) {
            self.bump();
        }
    }

    fn parse_object_literal(&mut self) -> Result<Expr, String> {
        let mut pairs = Vec::new();
        if !matches!(self.peek(), Token::RBrace) {
            loop {
                if matches!(self.peek(), Token::Ellipsis) {
                    self.bump();
                    let expr = self.parse_assign()?;
                    pairs.push((ObjectEntryKey::Spread(expr), Expr::Lit(Kv8Value::Undefined)));
                    if matches!(self.peek(), Token::Comma) {
                        self.bump();
                    } else {
                        break;
                    }
                    continue;
                }
                let is_async = if matches!(self.peek(), Token::Async) {
                    !matches!(self.tokens.get(self.pos + 1), Some(Token::Colon))
                } else {
                    false
                };
                if is_async {
                    self.bump();
                }
                let key = if matches!(self.peek(), Token::LBracket) {
                    self.bump();
                    let key_expr = self.parse_assign()?;
                    self.expect(Token::RBracket)?;
                    ObjectEntryKey::Computed(key_expr)
                } else if matches!(self.peek(), Token::Function) {
                    self.bump();
                    ObjectEntryKey::Lit("function".into())
                } else {
                    ObjectEntryKey::Lit(match self.bump() {
                        Token::Ident(name) => name,
                        Token::String(s) => s,
                        Token::Number(n) => {
                            if n.fract() == 0.0 {
                                format!("{}", n as i64)
                            } else {
                                n.to_string()
                            }
                        }
                        Token::Default => "default".into(),
                        Token::True => "true".into(),
                        Token::False => "false".into(),
                        Token::Null => "null".into(),
                        Token::Undefined => "undefined".into(),
                        Token::Async => "async".into(),
                        other => return Err(self.err(format!("object literal key: {:?}", other))),
                    })
                };
                let shorthand = matches!(self.peek(), Token::Comma | Token::RBrace);
                let val = if matches!(self.peek(), Token::LParen) {
                    self.bump();
                    let params = self.parse_params()?;
                    self.expect(Token::RParen)?;
                    let body = self.parse_block_or_stmt()?;
                    if is_async {
                        Expr::Lit(Kv8Value::Undefined) // TODO: async object method
                    } else {
                        Expr::FunExpr(params, body)
                    }
                } else if let ObjectEntryKey::Lit(name) = &key {
                    if shorthand {
                        Expr::Var(name.clone())
                    } else {
                        if is_async {
                            return Err("async object literal needs method".into());
                        }
                        self.expect(Token::Colon)?;
                        self.parse_assign()?
                    }
                } else {
                    if is_async {
                        return Err("async object literal needs method".into());
                    }
                    self.expect(Token::Colon)?;
                    self.parse_assign()?
                };
                pairs.push((key, val));
                if matches!(self.peek(), Token::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        self.expect(Token::RBrace)?;
        Ok(Expr::Object(pairs))
    }

    fn parse_array_literal(&mut self) -> Result<Expr, String> {
        let mut elems = Vec::new();
        if !matches!(self.peek(), Token::RBracket) {
            loop {
                elems.push(self.parse_assign()?);
                if matches!(self.peek(), Token::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        self.expect(Token::RBracket)?;
        Ok(Expr::Array(elems))
    }
}

fn expr_to_lvalue(expr: &Expr) -> Option<LValue> {
    match expr {
        Expr::Var(name) => Some(LValue::Name(name.clone())),
        Expr::This => Some(LValue::This),
        Expr::Member(base, field) => expr_to_lvalue(base)
            .map(|b| LValue::Member(Box::new(b), field.clone()))
            .or_else(|| Some(LValue::MemberExpr(base.clone(), field.clone()))),
        Expr::Index(base, idx) => expr_to_lvalue(base)
            .map(|b| LValue::Index(Box::new(b), idx.clone()))
            .or_else(|| Some(LValue::IndexExpr(base.clone(), idx.clone()))),
        _ => None,
    }
}

fn is_kv8_array(v: &Kv8Value) -> bool {
    matches!(
        v,
        Kv8Value::Obj(m) if m.get("__native").and_then(|v| v.as_str()) == Some("Array")
    )
}

fn array_length_of(map: &HashMap<String, Kv8Value>) -> usize {
    map.get("length")
        .and_then(|v| v.as_num())
        .unwrap_or(0.0)
        .max(0.0) as usize
}

fn array_values_of(map: &Kv8Value) -> Vec<Kv8Value> {
    let Kv8Value::Obj(m) = map else {
        return Vec::new();
    };
    let len = array_length_of(m);
    (0..len)
        .map(|i| {
            m.get(&i.to_string())
                .cloned()
                .unwrap_or(Kv8Value::Undefined)
        })
        .collect()
}

fn make_bound_function(
    target: Kv8Value,
    this_val: Kv8Value,
    preset_args: Vec<Kv8Value>,
) -> Kv8Value {
    let mut m = HashMap::new();
    m.insert("__native".into(), Kv8Value::Str("bound.call".into()));
    m.insert("__target".into(), target);
    m.insert("__this".into(), this_val);
    m.insert("__args".into(), array_from_values(preset_args));
    Kv8Value::Obj(m)
}

fn array_method(name: &str) -> Kv8Value {
    global_native(&format!("Array.{name}"))
}

fn array_from_values(values: Vec<Kv8Value>) -> Kv8Value {
    let mut m = HashMap::new();
    m.insert("__native".into(), Kv8Value::Str("Array".into()));
    let len = values.len();
    for (i, v) in values.into_iter().enumerate() {
        m.insert(i.to_string(), v);
    }
    m.insert("length".into(), Kv8Value::Num(len as f64));
    Kv8Value::Obj(m)
}

fn array_with_length(n: usize) -> Kv8Value {
    let mut m = HashMap::new();
    m.insert("__native".into(), Kv8Value::Str("Array".into()));
    m.insert("length".into(), Kv8Value::Num(n as f64));
    for i in 0..n {
        m.insert(i.to_string(), Kv8Value::Undefined);
    }
    Kv8Value::Obj(m)
}

fn is_map_entry_key(key: &str) -> bool {
    !key.starts_with("__")
}

fn set_storage_key(v: &Kv8Value) -> String {
    format!("\x01{}", index_to_key(v))
}

fn set_from_values(values: Vec<Kv8Value>) -> Kv8Value {
    let mut m = HashMap::new();
    m.insert("__native".into(), Kv8Value::Str("Set".into()));
    for v in values {
        m.insert(set_storage_key(&v), v);
    }
    Kv8Value::Obj(m)
}

fn map_from_entries(entries: Vec<(String, Kv8Value)>) -> Kv8Value {
    let mut m = HashMap::new();
    m.insert("__native".into(), Kv8Value::Str("Map".into()));
    for (k, v) in entries {
        m.insert(k, v);
    }
    Kv8Value::Obj(m)
}

fn weak_map_from_entries(entries: Vec<(String, Kv8Value)>) -> Kv8Value {
    let mut m = HashMap::new();
    m.insert("__native".into(), Kv8Value::Str("WeakMap".into()));
    for (k, v) in entries {
        m.insert(k, v);
    }
    Kv8Value::Obj(m)
}

fn construct_map_from_args(args: Vec<Kv8Value>) -> Result<Kv8Value, String> {
    let mut entries = Vec::new();
    if let Some(iterable) = args.first() {
        if is_kv8_array(iterable) {
            let Kv8Value::Obj(arr) = iterable else {
                return Ok(map_from_entries(entries));
            };
            let len = array_length_of(arr);
            for i in 0..len {
                let pair = arr.get(&i.to_string()).cloned().unwrap_or(Kv8Value::Undefined);
                if let Kv8Value::Obj(pair_m) = pair {
                    let k = pair_m
                        .get("0")
                        .cloned()
                        .unwrap_or(Kv8Value::Undefined);
                    let v = pair_m
                        .get("1")
                        .cloned()
                        .unwrap_or(Kv8Value::Undefined);
                    entries.push((index_to_key(&k), v));
                }
            }
        }
    }
    Ok(map_from_entries(entries))
}

fn construct_set_from_args(args: Vec<Kv8Value>) -> Result<Kv8Value, String> {
    let mut values = Vec::new();
    if let Some(iterable) = args.first() {
        if is_kv8_array(iterable) {
            let Kv8Value::Obj(arr) = iterable else {
                return Ok(set_from_values(values));
            };
            let len = array_length_of(arr);
            for i in 0..len {
                values.push(
                    arr.get(&i.to_string())
                        .cloned()
                        .unwrap_or(Kv8Value::Undefined),
                );
            }
        }
    }
    Ok(set_from_values(values))
}

fn map_namespace() -> Kv8Value {
    global_native("Map")
}

fn set_namespace() -> Kv8Value {
    global_native("Set")
}

fn form_data_namespace() -> Kv8Value {
    global_native("FormData")
}

fn date_namespace() -> Kv8Value {
    let mut now = HashMap::new();
    now.insert("__native".into(), Kv8Value::Str("Date.now".into()));
    let mut m = HashMap::new();
    m.insert("__native".into(), Kv8Value::Str("Date".into()));
    m.insert("now".into(), Kv8Value::Obj(now));
    Kv8Value::Obj(m)
}

fn string_namespace() -> Kv8Value {
    let mut from_char = HashMap::new();
    from_char.insert(
        "__native".into(),
        Kv8Value::Str("String.fromCharCode".into()),
    );
    let mut m = HashMap::new();
    m.insert("__native".into(), Kv8Value::Str("String".into()));
    m.insert("fromCharCode".into(), Kv8Value::Obj(from_char));
    Kv8Value::Obj(m)
}

fn math_namespace() -> Kv8Value {
    let mut m = HashMap::new();
    m.insert("__native".into(), Kv8Value::Str("Math".into()));
    m.insert("LN2".into(), Kv8Value::Num(std::f64::consts::LN_2));
    for name in ["random", "floor", "min", "log", "clz32"] {
        m.insert(name.into(), global_native(&format!("Math.{name}")));
    }
    Kv8Value::Obj(m)
}

fn performance_object() -> Kv8Value {
    let mut now = HashMap::new();
    now.insert("__native".into(), Kv8Value::Str("performance.now".into()));
    let mut entries = HashMap::new();
    entries.insert(
        "__native".into(),
        Kv8Value::Str("performance.getEntriesByType".into()),
    );
    let mut m = HashMap::new();
    m.insert("__native".into(), Kv8Value::Str("performance".into()));
    m.insert("now".into(), Kv8Value::Obj(now));
    m.insert("getEntriesByType".into(), Kv8Value::Obj(entries));
    Kv8Value::Obj(m)
}

fn map_method(name: &str) -> Kv8Value {
    global_native(&format!("Map.{name}"))
}

fn set_method(name: &str) -> Kv8Value {
    global_native(&format!("Set.{name}"))
}

fn regexp_method(name: &str) -> Kv8Value {
    global_native(&format!("RegExp.{name}"))
}

fn number_method(name: &str) -> Kv8Value {
    global_native(&format!("Number.{name}"))
}

fn string_method(name: &str) -> Kv8Value {
    global_native(&format!("String.{name}"))
}

fn num_to_radix(n: f64, radix: u32) -> String {
    let radix = radix.clamp(2, 36);
    let mut v = n.trunc().max(0.0) as u64;
    if v == 0 {
        return "0".into();
    }
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = String::new();
    while v > 0 {
        out.insert(0, DIGITS[(v % radix as u64) as usize] as char);
        v /= radix as u64;
    }
    out
}

fn js_str_len(s: &str) -> usize {
    s.chars().count()
}

fn js_str_index(s: &str, idx: f64) -> usize {
    let len = js_str_len(s) as i64;
    let mut i = idx.trunc() as i64;
    if i < 0 {
        i = len + i;
    }
    i.clamp(0, len) as usize
}

fn js_encode_uri_component(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

fn index_to_key(index: &Kv8Value) -> String {
    match index {
        Kv8Value::Num(n) => {
            if n.fract() == 0.0 {
                format!("{}", *n as i64)
            } else {
                n.to_string()
            }
        }
        Kv8Value::Str(s) => s.clone(),
        Kv8Value::Symbol { key, .. } => key.clone(),
        _ => kv8_value_to_display(index),
    }
}

fn index_get(ctx: &Kv8Context, obj: Kv8Value, index: Kv8Value) -> Result<Kv8Value, String> {
    let key = index_to_key(&index);
    match obj {
        Kv8Value::Dom(node) => ctx.with_mut(|inner| {
            Ok(inner
                .dom_expandos
                .get(&node.id)
                .and_then(|m| m.get(&key))
                .cloned()
                .unwrap_or(Kv8Value::Undefined))
        }),
        Kv8Value::Str(s) => {
            if let Ok(idx) = key.parse::<usize>() {
                Ok(s
                    .chars()
                    .nth(idx)
                    .map(|c| Kv8Value::Str(c.to_string()))
                    .unwrap_or(Kv8Value::Undefined))
            } else {
                Ok(Kv8Value::Undefined)
            }
        }
        Kv8Value::Obj(map) => {
            if let Ok(Some(v)) = object_property_get(ctx, &map, &key) {
                return Ok(v);
            }
            if is_kv8_array(&Kv8Value::Obj(map.clone())) {
                return Ok(map.get(&key).cloned().unwrap_or(Kv8Value::Undefined));
            }
            Ok(map.get(&key).cloned().unwrap_or(Kv8Value::Undefined))
        }
        _ => Ok(Kv8Value::Undefined),
    }
}

fn set_index_on_obj(map: &mut HashMap<String, Kv8Value>, key: &str, val: Kv8Value) {
    if map.get("__native").and_then(|v| v.as_str()) == Some("Array") {
        if let Ok(idx) = key.parse::<usize>() {
            let len = array_length_of(map);
            map.insert(key.to_string(), val);
            if idx + 1 > len {
                map.insert("length".into(), Kv8Value::Num((idx + 1) as f64));
            }
            return;
        }
    }
    map.insert(key.to_string(), val);
}

fn regex_object(pattern: &str, flags: &str) -> Kv8Value {
    let mut m = HashMap::new();
    m.insert("__native".into(), Kv8Value::Str("RegExp".into()));
    m.insert("source".into(), Kv8Value::Str(pattern.to_string()));
    m.insert("flags".into(), Kv8Value::Str(flags.to_string()));
    m.insert(
        "global".into(),
        Kv8Value::Bool(flags.contains('g')),
    );
    Kv8Value::Obj(m)
}

fn read_lvalue(ctx: &Kv8Context, lv: &LValue) -> Result<Kv8Value, String> {
    match lv {
        LValue::Name(name) => {
            if let Some(v) = ctx.with_mut(|inner| Ok(inner.scope_get(name)))? {
                return Ok(v);
            }
            eval_var(ctx, name)
        }
        LValue::This => current_this(ctx),
        LValue::Member(base, field) => {
            let v = read_lvalue(ctx, base)?;
            member_get(ctx, v, field)
        }
        LValue::Index(base, index_expr) => {
            let parent = read_lvalue(ctx, base)?;
            let idx = eval_expr(ctx, (*index_expr.as_ref()).clone())?;
            index_get(ctx, parent, idx)
        }
        LValue::MemberExpr(base, field) => {
            let v = eval_expr(ctx, (**base).clone())?;
            member_get(ctx, v, field)
        }
        LValue::IndexExpr(base, index_expr) => {
            let parent = eval_expr(ctx, (**base).clone())?;
            let idx = eval_expr(ctx, (**index_expr).clone())?;
            index_get(ctx, parent, idx)
        }
    }
}

fn eval_compound(op: char, cur: &Kv8Value, rhs: &Kv8Value) -> Result<Kv8Value, String> {
    Ok(match op {
        '+' => {
            if matches!(cur, Kv8Value::Str(_)) || matches!(rhs, Kv8Value::Str(_)) {
                Kv8Value::Str(format!(
                    "{}{}",
                    kv8_value_to_display(cur),
                    kv8_value_to_display(rhs)
                ))
            } else {
                Kv8Value::Num(cur.as_num().unwrap_or(0.0) + rhs.as_num().unwrap_or(0.0))
            }
        }
        '-' => Kv8Value::Num(cur.as_num().unwrap_or(0.0) - rhs.as_num().unwrap_or(0.0)),
        '*' => Kv8Value::Num(cur.as_num().unwrap_or(0.0) * rhs.as_num().unwrap_or(0.0)),
        '/' => Kv8Value::Num(cur.as_num().unwrap_or(0.0) / rhs.as_num().unwrap_or(1.0)),
        'A' | 'o' | '^' | 'L' | 'R' | 'U' => eval_bin(op, cur, rhs),
        _ => return Err("unsupported compound assign".into()),
    })
}

fn for_in_keys(v: &Kv8Value) -> Vec<String> {
    match v {
        Kv8Value::Obj(m) => {
            if is_kv8_array(v) {
                let len = array_length_of(m);
                return (0..len).map(|i| i.to_string()).collect();
            }
            m.keys()
                .filter(|k| !k.starts_with("__"))
                .cloned()
                .collect()
        }
        _ => Vec::new(),
    }
}

fn is_nullish(v: &Kv8Value) -> bool {
    matches!(v, Kv8Value::Null | Kv8Value::Undefined)
}

fn for_of_values(ctx: &Kv8Context, v: &Kv8Value) -> Result<Vec<Kv8Value>, String> {
    match v {
        Kv8Value::Str(s) => Ok(s.chars().map(|c| Kv8Value::Str(c.to_string())).collect()),
        Kv8Value::Obj(m) if is_kv8_array(v) => Ok(array_values_of(&Kv8Value::Obj(m.clone()))),
        Kv8Value::Obj(m) => {
            if let Some(iter_method) = m.get("Symbol.iterator").or_else(|| m.get("[Symbol.iterator]")) {
                let iter = call_value(ctx, iter_method.clone(), vec![v.clone()])?;
                return iterator_to_values(ctx, iter);
            }
            Ok(Vec::new())
        }
        _ => Ok(Vec::new()),
    }
}

fn iterator_to_values(ctx: &Kv8Context, iter: Kv8Value) -> Result<Vec<Kv8Value>, String> {
    let mut out = Vec::new();
    for _ in 0..10_000 {
        let next_fn = member_get(ctx, iter.clone(), "next")?;
        let step = call_value(ctx, next_fn, vec![])?;
        let done = member_get(ctx, step.clone(), "done")?;
        if done.is_truthy() {
            break;
        }
        out.push(member_get(ctx, step, "value")?);
    }
    Ok(out)
}

fn exec_import(ctx: &Kv8Context, default: Option<String>, named: Vec<String>, from: String) -> Result<(), String> {
    let module = ctx.with_mut(|inner| {
        inner
            .modules
            .get(&from)
            .cloned()
            .ok_or_else(|| format!("module not found: {from}"))
    })?;
    ctx.with_mut(|inner| {
        if let Some(name) = default {
            let val = module
                .default_export
                .clone()
                .ok_or_else(|| format!("module {from} has no default export"))?;
            inner.scope_current_mut().insert(name, val);
        }
        for name in named {
            let val = module
                .named
                .get(&name)
                .cloned()
                .ok_or_else(|| format!("module {from} has no export {name}"))?;
            inner.scope_current_mut().insert(name, val);
        }
        Ok(())
    })?;
    Ok(())
}

pub fn register_kv8_module(
    ctx: &Kv8Context,
    name: &str,
    default_export: Option<Kv8Value>,
    named: HashMap<String, Kv8Value>,
) -> Result<(), String> {
    ctx.with_mut(|inner| {
        inner.modules.insert(
            name.into(),
            Kv8Module {
                default_export,
                named,
            },
        );
        Ok(())
    })?;
    Ok(())
}

fn exec_labeled(ctx: &Kv8Context, name: String, inner: &Stmt) -> Result<Flow, String> {
    let label = name.as_str();
    match inner {
        Stmt::ForClassic(init, cond, update, body) => {
            run_stmts(ctx, &init)?;
            let mut last = Kv8Value::Undefined;
            let mut iter = 0u64;
            loop {
                if let Some(c) = &cond {
                    if !eval_expr(ctx, c.clone())?.is_truthy() {
                        break;
                    }
                }
                iter += 1;
                match run_stmts(ctx, &body)? {
                    Flow::Break(l) if flow_break_matches(&l, Some(label)) => break,
                    Flow::Break(l) => return Ok(Flow::Break(l)),
                    Flow::Continue(l) if flow_continue_matches(&l, Some(label)) => {}
                    Flow::Continue(l) => return Ok(Flow::Continue(l)),
                    Flow::Return(v) => return Ok(Flow::Return(v)),
                    Flow::Throw(v) => return Ok(Flow::Throw(v)),
                    Flow::Next(v) => last = v,
                }
                if let Some(u) = &update {
                    eval_expr(ctx, u.clone())?;
                }
                if iter > 10_000 {
                    break;
                }
            }
            Ok(Flow::Next(last))
        }
        Stmt::While(cond, body) => {
            let mut last = Kv8Value::Undefined;
            let mut iter = 0u64;
            loop {
                if !eval_expr(ctx, cond.clone())?.is_truthy() {
                    break;
                }
                match run_stmts(ctx, &body)? {
                    Flow::Break(l) if flow_break_matches(&l, Some(label)) => break,
                    Flow::Break(l) => return Ok(Flow::Break(l)),
                    Flow::Continue(l) if flow_continue_matches(&l, Some(label)) => {}
                    Flow::Continue(l) => return Ok(Flow::Continue(l)),
                    Flow::Return(v) => return Ok(Flow::Return(v)),
                    Flow::Throw(v) => return Ok(Flow::Throw(v)),
                    Flow::Next(v) => last = v,
                }
                iter += 1;
                if iter > 10_000 {
                    break;
                }
            }
            Ok(Flow::Next(last))
        }
        Stmt::DoWhile(body, cond) => {
            let mut last = Kv8Value::Undefined;
            let mut iter = 0u64;
            loop {
                match run_stmts(ctx, &body)? {
                    Flow::Break(l) if flow_break_matches(&l, Some(label)) => break,
                    Flow::Break(l) => return Ok(Flow::Break(l)),
                    Flow::Continue(l) if flow_continue_matches(&l, Some(label)) => {}
                    Flow::Continue(l) => return Ok(Flow::Continue(l)),
                    Flow::Return(v) => return Ok(Flow::Return(v)),
                    Flow::Throw(v) => return Ok(Flow::Throw(v)),
                    Flow::Next(v) => last = v,
                }
                if !eval_expr(ctx, cond.clone())?.is_truthy() {
                    break;
                }
                iter += 1;
                if iter > 10_000 {
                    break;
                }
            }
            Ok(Flow::Next(last))
        }
        Stmt::For(var, start, cond, step, body) => {
            let start_v = eval_expr(ctx, start.clone())?;
            ctx.with_mut(|inner| {
                inner.scope_current_mut().insert(var.clone(), start_v);
                Ok(())
            })?;
            let mut last = Kv8Value::Undefined;
            let mut iter = 0u64;
            loop {
                if !eval_expr(ctx, cond.clone())?.is_truthy() {
                    break;
                }
                iter += 1;
                match run_stmts(ctx, body)? {
                    Flow::Break(l) if flow_break_matches(&l, Some(label)) => break,
                    Flow::Break(l) => return Ok(Flow::Break(l)),
                    Flow::Continue(l) if flow_continue_matches(&l, Some(label)) => {}
                    Flow::Continue(l) => return Ok(Flow::Continue(l)),
                    Flow::Return(v) => return Ok(Flow::Return(v)),
                    Flow::Throw(v) => return Ok(Flow::Throw(v)),
                    Flow::Next(v) => last = v,
                }
                let step_v = eval_expr(ctx, step.clone())?;
                ctx.with_mut(|inner| {
                    inner.scope_current_mut().insert(var.clone(), step_v);
                    Ok(())
                })?;
                if iter > 10_000 {
                    break;
                }
            }
            Ok(Flow::Next(last))
        }
        Stmt::ForIn(lv, iterable, body) => {
            let obj = eval_expr(ctx, iterable.clone())?;
            let keys = for_in_keys(&obj);
            let mut last = Kv8Value::Undefined;
            for key in keys {
                assign_lvalue(ctx, lv.clone(), Kv8Value::Str(key))?;
                match run_stmts(ctx, body)? {
                    Flow::Break(l) if flow_break_matches(&l, Some(label)) => {
                        return Ok(Flow::Next(last));
                    }
                    Flow::Break(l) => return Ok(Flow::Break(l)),
                    Flow::Continue(l) if flow_continue_matches(&l, Some(label)) => continue,
                    Flow::Continue(l) => return Ok(Flow::Continue(l)),
                    Flow::Return(v) => return Ok(Flow::Return(v)),
                    Flow::Throw(v) => return Ok(Flow::Throw(v)),
                    Flow::Next(v) => last = v,
                }
            }
            Ok(Flow::Next(last))
        }
        Stmt::ForOf(lv, iterable, body) => {
            let obj = eval_expr(ctx, iterable.clone())?;
            let values = for_of_values(ctx, &obj)?;
            let mut last = Kv8Value::Undefined;
            for val in values {
                assign_lvalue(ctx, lv.clone(), val)?;
                match run_stmts(ctx, body)? {
                    Flow::Break(l) if flow_break_matches(&l, Some(label)) => {
                        return Ok(Flow::Next(last));
                    }
                    Flow::Break(l) => return Ok(Flow::Break(l)),
                    Flow::Continue(l) if flow_continue_matches(&l, Some(label)) => continue,
                    Flow::Continue(l) => return Ok(Flow::Continue(l)),
                    Flow::Return(v) => return Ok(Flow::Return(v)),
                    Flow::Throw(v) => return Ok(Flow::Throw(v)),
                    Flow::Next(v) => last = v,
                }
            }
            Ok(Flow::Next(last))
        }
        Stmt::Switch(disc, cases, default) => {
            let disc_val = eval_expr(ctx, disc.clone())?;
            let start = cases.iter().position(|case| {
                eval_expr(ctx, case.label.clone())
                    .map(|lbl| kv8_strict_eq(&disc_val, &lbl))
                    .unwrap_or(false)
            });
            if let Some(idx) = start {
                for case in &cases[idx..] {
                    match run_stmts(ctx, &case.body)? {
                        Flow::Break(l) if flow_break_matches(&l, Some(label)) => {
                            return Ok(Flow::Next(Kv8Value::Undefined));
                        }
                        Flow::Break(l) => return Ok(Flow::Break(l)),
                        Flow::Continue(l) if flow_continue_matches(&l, Some(label)) => {}
                        Flow::Continue(l) => return Ok(Flow::Continue(l)),
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        Flow::Throw(v) => return Ok(Flow::Throw(v)),
                        Flow::Next(_) => {}
                    }
                }
            } else if let Some(def) = default {
                run_stmts(ctx, &def)?;
            }
            Ok(Flow::Next(Kv8Value::Undefined))
        }
        Stmt::Block(stmts) => match run_stmts(ctx, &stmts)? {
            Flow::Break(Some(l)) if l == name => Ok(Flow::Next(Kv8Value::Undefined)),
            other => Ok(other),
        },
        _ => Err(format!("invalid label target: {name}")),
    }
}

fn exec_stmt(ctx: &Kv8Context, stmt: &Stmt) -> Result<Flow, String> {
    match stmt {
        Stmt::Var(name, expr) | Stmt::Let(name, expr) => {
            let val = eval_expr(ctx, expr.clone())?;
            ctx.with_mut(|inner| {
                inner.scope_current_mut().insert(name.clone(), val.clone());
                Ok(())
            })?;
            Ok(Flow::Next(val))
        }
        Stmt::Assign(lv, expr) => {
            let val = eval_expr(ctx, expr.clone())?;
            assign_lvalue(ctx, lv.clone(), val.clone())?;
            Ok(Flow::Next(val))
        }
        Stmt::Return(expr) => Ok(Flow::Return(eval_expr(ctx, expr.clone())?)),
        Stmt::Expr(expr) => Ok(Flow::Next(eval_expr(ctx, expr.clone())?)),
        Stmt::Break(label) => Ok(Flow::Break(label.clone())),
        Stmt::Continue(label) => Ok(Flow::Continue(label.clone())),
        Stmt::Block(stmts) => run_stmts(ctx, stmts),
        Stmt::DoWhile(body, cond) => {
            let mut last = Kv8Value::Undefined;
            let mut iter = 0u64;
            loop {
                match run_stmts(ctx, body)? {
                    Flow::Break(l) if flow_break_matches(&l, None) => break,
                    Flow::Break(l) => return Ok(Flow::Break(l)),
                    Flow::Continue(l) if flow_continue_matches(&l, None) => {}
                    Flow::Continue(l) => return Ok(Flow::Continue(l)),
                    Flow::Return(v) => return Ok(Flow::Return(v)),
                    Flow::Throw(v) => return Ok(Flow::Throw(v)),
                    Flow::Next(v) => last = v,
                }
                if !eval_expr(ctx, cond.clone())?.is_truthy() {
                    break;
                }
                iter += 1;
                if iter > 10_000 {
                    break;
                }
            }
            Ok(Flow::Next(last))
        }
        Stmt::Label(name, inner) => exec_labeled(ctx, name.clone(), inner),
        Stmt::Throw(expr) => Ok(Flow::Throw(eval_expr(ctx, expr.clone())?)),
        Stmt::If(cond, then_b, else_b) => {
            if eval_expr(ctx, cond.clone())?.is_truthy() {
                run_stmts(ctx, then_b)
            } else if let Some(e) = else_b {
                run_stmts(ctx, e)
            } else {
                Ok(Flow::Next(Kv8Value::Undefined))
            }
        }
        Stmt::For(var, start, cond, step, body) => {
            let start_v = eval_expr(ctx, start.clone())?;
            ctx.with_mut(|inner| {
                inner.scope_current_mut().insert(var.clone(), start_v);
                Ok(())
            })?;
            let jit_ok = !stmts_have_loop_control(body);
            let key = loop_key(var, &super::opt::expr_key(cond));
            let mut last = Kv8Value::Undefined;
            let mut iter = 0u64;
            let mut body_refs = std::collections::HashSet::new();
            collect_vars_stmts(body, &mut body_refs);
            body_refs.insert(var.clone());
            loop {
                if !eval_expr(ctx, cond.clone())?.is_truthy() {
                    break;
                }
                iter += 1;
                let body_flow = if jit_ok {
                    let jit_fn = ctx.with_mut(|inner| {
                        let jit = inner.jit.get_or_insert_with(Default::default);
                        if jit.record_loop(&key) {
                            let _ = jit.compile_loop(&key, body);
                        }
                        Ok(jit.get_loop(&key).cloned())
                    })?;
                    if let Some(f) = jit_fn.filter(|_| iter > JIT_THRESHOLD) {
                        let mut env = crate::value::Environment::new();
                        sync_scope_to_env(ctx, &body_refs, &mut env)?;
                        let out = run_kv8_bytecode_fn(&f, vec![], &mut env)?;
                        sync_env_to_scope(ctx, &body_refs, &env)?;
                        Flow::Next(kabootar_to_kv8(out))
                    } else {
                        run_stmts(ctx, body)?
                    }
                } else {
                    run_stmts(ctx, body)?
                };
                match body_flow {
                    Flow::Break(l) if flow_break_matches(&l, None) => break,
                    Flow::Break(l) => return Ok(Flow::Break(l)),
                    Flow::Continue(l) if flow_continue_matches(&l, None) => {}
                    Flow::Continue(l) => return Ok(Flow::Continue(l)),
                    Flow::Return(v) => return Ok(Flow::Return(v)),
                    Flow::Throw(v) => return Ok(Flow::Throw(v)),
                    Flow::Next(v) => last = v,
                }
                let step_v = eval_expr(ctx, step.clone())?;
                ctx.with_mut(|inner| {
                    inner.scope_current_mut().insert(var.clone(), step_v);
                    Ok(())
                })?;
                if iter > 10_000 {
                    break;
                }
            }
            Ok(Flow::Next(last))
        }
        Stmt::While(cond, body) => {
            let mut last = Kv8Value::Undefined;
            let mut iter = 0u64;
            loop {
                if !eval_expr(ctx, cond.clone())?.is_truthy() {
                    break;
                }
                match run_stmts(ctx, &body)? {
                    Flow::Break(l) if flow_break_matches(&l, None) => break,
                    Flow::Break(l) => return Ok(Flow::Break(l)),
                    Flow::Continue(l) if flow_continue_matches(&l, None) => {}
                    Flow::Continue(l) => return Ok(Flow::Continue(l)),
                    Flow::Return(v) => return Ok(Flow::Return(v)),
                    Flow::Throw(v) => return Ok(Flow::Throw(v)),
                    Flow::Next(v) => last = v,
                }
                iter += 1;
                if iter > 10_000 {
                    break;
                }
            }
            Ok(Flow::Next(last))
        }
        Stmt::Switch(disc, cases, default) => {
            let disc_val = eval_expr(ctx, disc.clone())?;
            let start = cases.iter().position(|case| {
                eval_expr(ctx, case.label.clone())
                    .map(|label| kv8_strict_eq(&disc_val, &label))
                    .unwrap_or(false)
            });
            if let Some(idx) = start {
                for case in &cases[idx..] {
                    match run_stmts(ctx, &case.body)? {
                        Flow::Break(l) if flow_break_matches(&l, None) => break,
                        Flow::Break(l) => return Ok(Flow::Break(l)),
                        Flow::Continue(l) if flow_continue_matches(&l, None) => {}
                        Flow::Continue(l) => return Ok(Flow::Continue(l)),
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        Flow::Throw(v) => return Ok(Flow::Throw(v)),
                        Flow::Next(_) => {}
                    }
                }
            } else if let Some(def) = default {
                run_stmts(ctx, &def)?;
            }
            Ok(Flow::Next(Kv8Value::Undefined))
        }
        Stmt::ForClassic(init, cond, update, body) => {
            run_stmts(ctx, &init)?;
            let mut last = Kv8Value::Undefined;
            let mut iter = 0u64;
            loop {
                if let Some(c) = &cond {
                    if !eval_expr(ctx, c.clone())?.is_truthy() {
                        break;
                    }
                }
                iter += 1;
                match run_stmts(ctx, &body)? {
                    Flow::Break(l) if flow_break_matches(&l, None) => break,
                    Flow::Break(l) => return Ok(Flow::Break(l)),
                    Flow::Continue(l) if flow_continue_matches(&l, None) => {}
                    Flow::Continue(l) => return Ok(Flow::Continue(l)),
                    Flow::Return(v) => return Ok(Flow::Return(v)),
                    Flow::Throw(v) => return Ok(Flow::Throw(v)),
                    Flow::Next(v) => last = v,
                }
                if let Some(u) = update {
                    eval_expr(ctx, u.clone())?;
                }
                if iter > 10_000 {
                    break;
                }
            }
            Ok(Flow::Next(last))
        }
        Stmt::ForIn(lv, iterable, body) => {
            let obj = eval_expr(ctx, iterable.clone())?;
            let keys = for_in_keys(&obj);
            let mut last = Kv8Value::Undefined;
            for key in keys {
                assign_lvalue(ctx, lv.clone(), Kv8Value::Str(key))?;
                match run_stmts(ctx, body)? {
                    Flow::Break(l) if flow_break_matches(&l, None) => break,
                    Flow::Break(l) => return Ok(Flow::Break(l)),
                    Flow::Continue(l) if flow_continue_matches(&l, None) => continue,
                    Flow::Continue(l) => return Ok(Flow::Continue(l)),
                    Flow::Return(v) => return Ok(Flow::Return(v)),
                    Flow::Throw(v) => return Ok(Flow::Throw(v)),
                    Flow::Next(v) => last = v,
                }
            }
            Ok(Flow::Next(last))
        }
        Stmt::ForOf(lv, iterable, body) => {
            let obj = eval_expr(ctx, iterable.clone())?;
            let values = for_of_values(ctx, &obj)?;
            let mut last = Kv8Value::Undefined;
            for val in values {
                assign_lvalue(ctx, lv.clone(), val)?;
                match run_stmts(ctx, body)? {
                    Flow::Break(l) if flow_break_matches(&l, None) => break,
                    Flow::Break(l) => return Ok(Flow::Break(l)),
                    Flow::Continue(l) if flow_continue_matches(&l, None) => continue,
                    Flow::Continue(l) => return Ok(Flow::Continue(l)),
                    Flow::Return(v) => return Ok(Flow::Return(v)),
                    Flow::Throw(v) => return Ok(Flow::Throw(v)),
                    Flow::Next(v) => last = v,
                }
            }
            Ok(Flow::Next(last))
        }
        Stmt::Import {
            default,
            named,
            from,
        } => {
            exec_import(ctx, default.clone(), named.clone(), from.clone())?;
            Ok(Flow::Next(Kv8Value::Undefined))
        }
        Stmt::ExportDefault(expr) => {
            let val = eval_expr(ctx, expr.clone())?;
            ctx.with_mut(|inner| {
                inner.export_default = Some(val);
                Ok(())
            })?;
            Ok(Flow::Next(Kv8Value::Undefined))
        }
        Stmt::ExportNamed(names) => {
            for name in names {
                let val = eval_var(ctx, name)?;
                ctx.with_mut(|inner| {
                    inner.export_bindings.insert(name.clone(), val.clone());
                    Ok(())
                })?;
            }
            Ok(Flow::Next(Kv8Value::Undefined))
        }
        Stmt::TryCatch(try_body, catch_clause, finally_body) => {
            let mut pending_throw = None;
            let flow = match run_stmts(ctx, &try_body)? {
                Flow::Throw(err) => {
                    if let Some((catch_var, catch_body)) = &catch_clause {
                        ctx.with_mut(|inner| {
                            inner
                                .scope_current_mut()
                                .insert(catch_var.clone(), err);
                            Ok(())
                        })?;
                        run_stmts(ctx, catch_body)?
                    } else {
                        pending_throw = Some(err);
                        Flow::Next(Kv8Value::Undefined)
                    }
                }
                other => other,
            };
            let Some(fin) = finally_body else {
                if let Some(err) = pending_throw {
                    return Ok(Flow::Throw(err));
                }
                return Ok(flow);
            };
            let fin_flow = run_stmts(ctx, &fin)?;
            if let Some(err) = pending_throw {
                return Ok(Flow::Throw(err));
            }
            Ok(match fin_flow {
                Flow::Throw(v) => Flow::Throw(v),
                Flow::Return(v) => Flow::Return(v),
                Flow::Break(l) => Flow::Break(l),
                Flow::Continue(l) => Flow::Continue(l),
                Flow::Next(_) => flow,
            })
        }
        Stmt::Function(name, params, body) => {
            let fun = new_fun(ctx, params.clone(), body.clone())?;
            ctx.with_mut(|inner| {
                inner.scope_current_mut().insert(name.clone(), fun);
                Ok(Flow::Next(Kv8Value::Undefined))
            })
        }
        Stmt::AsyncFunction(name, params, body) => {
            let fun = new_async_fun(ctx, params.clone(), body.clone())?;
            ctx.with_mut(|inner| {
                inner.scope_current_mut().insert(name.clone(), fun);
                Ok(Flow::Next(Kv8Value::Undefined))
            })
        }
    }
}

fn run_stmts(ctx: &Kv8Context, stmts: &[Stmt]) -> Result<Flow, String> {
    ctx.with_mut(|inner| {
        inner.exec_stmts_stack.push(super::context::ExecStmtsFrame {
            stmts: stmts.to_vec(),
            index: 0,
        });
        Ok(())
    })?;
    let mut last = Kv8Value::Undefined;
    let result = (|| {
        for (i, s) in stmts.iter().enumerate() {
            ctx.with_mut(|inner| {
                if let Some(frame) = inner.exec_stmts_stack.last_mut() {
                    frame.index = i;
                }
                Ok(())
            })?;
            match exec_stmt(ctx, s)? {
                Flow::Next(v) => last = v,
                other => return Ok(other),
            }
        }
        Ok(Flow::Next(last))
    })();
    ctx.with_mut(|inner| {
        inner.exec_stmts_stack.pop();
        Ok(())
    })?;
    result
}

fn new_fun(ctx: &Kv8Context, params: Vec<Kv8Param>, body: Vec<Stmt>) -> Result<Kv8Value, String> {
    let closure = ctx.with_mut(|inner| Ok(inner.capture_lexical_env()))?;
    Ok(Kv8Value::Fun {
        params,
        body,
        prototype: HashMap::new(),
        closure,
    })
}

fn new_async_fun(
    ctx: &Kv8Context,
    params: Vec<Kv8Param>,
    body: Vec<Stmt>,
) -> Result<Kv8Value, String> {
    let closure = ctx.with_mut(|inner| Ok(inner.capture_lexical_env()))?;
    Ok(Kv8Value::AsyncFun {
        params,
        body,
        prototype: HashMap::new(),
        closure,
    })
}

fn new_arrow(ctx: &Kv8Context, params: Vec<Kv8Param>, body: Box<Expr>) -> Result<Kv8Value, String> {
    let closure = ctx.with_mut(|inner| Ok(inner.capture_lexical_env()))?;
    Ok(Kv8Value::Arrow {
        params,
        body,
        closure,
    })
}

fn closure_snapshot_differs_from_outer(
    inner: &Kv8ContextInner,
    name: &str,
    captured: &Kv8Value,
) -> bool {
    match inner.scope_get_outer(name) {
        Some(outer) => !kv8_values_shallow_eq(captured, &outer),
        None => true,
    }
}

fn kv8_values_shallow_eq(a: &Kv8Value, b: &Kv8Value) -> bool {
    match (a, b) {
        (Kv8Value::Undefined, Kv8Value::Undefined) | (Kv8Value::Null, Kv8Value::Null) => true,
        (Kv8Value::Bool(x), Kv8Value::Bool(y)) => x == y,
        (Kv8Value::Num(x), Kv8Value::Num(y)) => x == y,
        (Kv8Value::Str(x), Kv8Value::Str(y)) => x == y,
        (Kv8Value::Obj(x), Kv8Value::Obj(y)) => match (
            x.get("__obj_id").and_then(|v| v.as_num()),
            y.get("__obj_id").and_then(|v| v.as_num()),
        ) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        },
        _ => false,
    }
}

fn bind_closure_env(ctx: &Kv8Context, closure: &HashMap<String, Kv8Value>) -> Result<(), String> {
    if closure.is_empty() {
        return Ok(());
    }
    ctx.with_mut(|inner| {
        let to_bind: Vec<(String, Kv8Value)> = closure
            .iter()
            .filter(|(k, v)| {
                !matches!(v, Kv8Value::Undefined)
                    && closure_snapshot_differs_from_outer(inner, k, v)
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let frame = inner.scope_current_mut();
        for (k, v) in to_bind {
            frame.insert(k, v);
        }
        Ok(())
    })
}

fn fun_prototype_map_mut(fun: &mut Kv8Value) -> Option<&mut HashMap<String, Kv8Value>> {
    match fun {
        Kv8Value::Fun { prototype, .. } | Kv8Value::AsyncFun { prototype, .. } => Some(prototype),
        _ => None,
    }
}

fn parent_tag(v: &Kv8Value) -> &'static str {
    match v {
        Kv8Value::Undefined => "undefined",
        Kv8Value::Null => "null",
        Kv8Value::Bool(_) => "bool",
        Kv8Value::Num(_) => "number",
        Kv8Value::Str(_) => "string",
        Kv8Value::Dom(_) => "dom",
        Kv8Value::Obj(_) => "object",
        Kv8Value::Fun { .. } => "function",
        Kv8Value::Arrow { .. } => "arrow",
        Kv8Value::AsyncFun { .. } => "async-function",
        Kv8Value::Promise(_) => "promise",
        Kv8Value::Symbol { .. } => "symbol",
    }
}

fn assign_fun_prototype(ctx: &Kv8Context, var: &str, val: Kv8Value) -> Result<bool, String> {
    ctx.with_mut(|inner| {
        let Some(fun) = inner.scope_resolve_mut(var) else {
            return Ok(false);
        };
        let Some(proto) = fun_prototype_map_mut(fun) else {
            return Ok(false);
        };
        *proto = match val {
            Kv8Value::Obj(m) => m,
            _ => HashMap::new(),
        };
        Ok(true)
    })
}

fn assign_fun_prototype_field(
    ctx: &Kv8Context,
    var: &str,
    field: &str,
    val: Kv8Value,
) -> Result<(), String> {
    ctx.with_mut(|inner| {
        let fun = inner
            .scope_resolve_mut(var)
            .ok_or_else(|| format!("cannot assign to undefined {var}"))?;
        let proto = fun_prototype_map_mut(fun).ok_or("cannot assign to member")?;
        proto.insert(field.to_string(), val);
        Ok(())
    })
}

fn assign_scope_var_member(
    ctx: &Kv8Context,
    var: &str,
    field: &str,
    val: Kv8Value,
) -> Result<(), String> {
    if var == "globalThis" || var == "self" {
        return set_global_prop(ctx, field, val);
    }
    ctx.with_mut(|inner| {
        let cell = inner
            .scope_resolve_mut(var)
            .ok_or_else(|| format!("cannot assign to undefined {var}"))?;
        match cell {
            Kv8Value::Obj(m) => {
                if let Some(id) = plain_obj_id(m) {
                    inner
                        .obj_store
                        .entry(id)
                        .or_default()
                        .insert(field.to_string(), val);
                } else {
                    m.insert(field.to_string(), val);
                }
                Ok(())
            }
            _ => Err("cannot assign to member of non-object".into()),
        }
    })
}

fn delete_lvalue(ctx: &Kv8Context, lv: &LValue) -> Result<bool, String> {
    match lv {
        LValue::Name(name) => ctx.with_mut(|inner| Ok(inner.scope_resolve_mut(name).is_none())),
        LValue::Member(base, field) => {
            let obj = read_lvalue(ctx, base)?;
            match obj {
                Kv8Value::Obj(mut m) => Ok(m.remove(field).is_some()),
                _ => Ok(true),
            }
        }
        LValue::MemberExpr(base, field) => {
            let obj = eval_expr(ctx, *base.clone())?;
            match obj {
                Kv8Value::Obj(mut m) => Ok(m.remove(field).is_some()),
                _ => Ok(true),
            }
        }
        LValue::Index(base, idx) => {
            let obj = read_lvalue(ctx, base)?;
            let key = index_to_key(&eval_expr(ctx, *idx.clone())?);
            match obj {
                Kv8Value::Obj(mut m) => Ok(m.remove(&key).is_some()),
                _ => Ok(true),
            }
        }
        LValue::IndexExpr(base, idx) => {
            let obj = eval_expr(ctx, *base.clone())?;
            let key = index_to_key(&eval_expr(ctx, *idx.clone())?);
            match obj {
                Kv8Value::Obj(mut m) => Ok(m.remove(&key).is_some()),
                _ => Ok(true),
            }
        }
        LValue::This => Ok(false),
    }
}

fn assign_lvalue(ctx: &Kv8Context, lv: LValue, val: Kv8Value) -> Result<(), String> {
    match lv {
        LValue::This => Err("cannot assign to this".into()),
        LValue::Member(base, field) if matches!(*base, LValue::This) => {
            write_this_member(ctx, &field, val)
        }
        LValue::Index(base, index_expr) if matches!(base.as_ref(), LValue::This) => {
            let key = index_to_key(&eval_expr(ctx, (*index_expr).clone())?);
            write_this_index(ctx, &key, val)
        }
        LValue::Name(name) => ctx.with_mut(|inner| {
            if let Some(cell) = inner.scope_resolve_mut(&name) {
                *cell = val;
            } else {
                inner.scope_current_mut().insert(name, val);
            }
            Ok(())
        }),
        LValue::Member(base, field) if matches!(base.as_ref(), LValue::Name(_)) && field == "prototype" => {
            let LValue::Name(var) = base.as_ref() else {
                unreachable!()
            };
            if assign_fun_prototype(ctx, var, val.clone())? {
                Ok(())
            } else {
                assign_scope_var_member(ctx, var, field.as_str(), val)
            }
        }
        LValue::Member(base, field)
            if matches!(
                base.as_ref(),
                LValue::Member(inner, proto) if matches!(inner.as_ref(), LValue::Name(_)) && proto == "prototype"
            ) =>
        {
            let LValue::Member(inner, _) = base.as_ref() else {
                unreachable!()
            };
            let LValue::Name(var) = inner.as_ref() else {
                unreachable!()
            };
            assign_fun_prototype_field(ctx, var, field.as_str(), val)
        }
        LValue::Member(base, field) => {
            if let LValue::Name(ref var) = *base {
                if var == "globalThis" || var == "self" {
                    return set_global_prop(ctx, &field, val);
                }
                let is_plain_obj = ctx.with_mut(|inner| {
                    Ok(matches!(
                        inner.scope_get(var),
                        Some(Kv8Value::Obj(m)) if m
                            .get("__native")
                            .and_then(|v| v.as_str())
                            .is_none_or(|n| !n.starts_with("element."))
                    ))
                })?;
                if is_plain_obj {
                    return assign_scope_var_member(ctx, var, field.as_str(), val);
                }
            }
            if let LValue::Member(ref el_lv, ref style_field) = *base {
                if style_field == "style" {
                    let el = eval_lvalue_as_obj(ctx, (**el_lv).clone())?;
                    if let Kv8Value::Dom(node) = &el {
                        if let Kv8Value::Str(s) = &val {
                            let _ = ctx.set_attr(node.id, &format!("style:{field}"), s);
                        }
                    }
                    if let LValue::Name(_var) = &**el_lv {
                        if let Kv8Value::Dom(node) = el {
                            if let Kv8Value::Str(s) = val {
                                let mut node = node;
                                node.set_attr(&format!("style:{field}"), &s);
                                write_dom_binding(ctx, (**el_lv).clone(), node)?;
                            }
                            return Ok(());
                        }
                    }
                    return Ok(());
                }
            }
            let parent = eval_lvalue_as_obj(ctx, *base.clone())?;
            let tag = parent_tag(&parent);
            match (parent, field.as_str()) {
                (Kv8Value::Dom(mut node), "textContent") => {
                    if let Kv8Value::Str(s) = val {
                        node.text = Some(s);
                        write_dom_binding(ctx, *base, node)?;
                    }
                    Ok(())
                }
                (Kv8Value::Dom(mut node), "innerHTML") => {
                    if let Kv8Value::Str(s) = val {
                        node.children = ctx.parse_inner_html_fragment(&s)?;
                        for child in &mut node.children {
                            assign_ids(child);
                        }
                        write_dom_binding(ctx, *base, node)?;
                    }
                    Ok(())
                }
                (Kv8Value::Dom(mut node), "id") => {
                    if let Kv8Value::Str(s) = val {
                        node.set_attr("id", &s);
                        write_dom_binding(ctx, *base, node)?;
                    }
                    Ok(())
                }
                _ => Err(format!("cannot assign to member .{field} on {tag}")),
            }
        }
        LValue::Index(base, index_expr) => {
            let key = index_to_key(&eval_expr(ctx, (*index_expr).clone())?);
            match base.as_ref() {
                LValue::Name(var) => ctx.with_mut(|inner| {
                    let cell = inner
                        .scope_resolve_mut(var)
                        .ok_or_else(|| format!("cannot assign to undefined {var}"))?;
                    match cell {
                        Kv8Value::Obj(m) => {
                            set_index_on_obj(m, &key, val);
                            Ok(())
                        }
                        Kv8Value::Dom(node) => {
                            let id = node.id;
                            inner
                                .dom_expandos
                                .entry(id)
                                .or_default()
                                .insert(key, val);
                            Ok(())
                        }
                        _ => Err(format!(
                            "cannot assign index on non-object ({var}={})",
                            callee_debug_hint(cell)
                        )),
                    }
                }),
                other => {
                    let parent = read_lvalue(ctx, other)?;
                    match parent {
                        Kv8Value::Obj(mut m) => {
                            set_index_on_obj(&mut m, &key, val.clone());
                            assign_lvalue(ctx, (*base).clone(), Kv8Value::Obj(m))
                        }
                        Kv8Value::Dom(node) => ctx.with_mut(|inner| {
                            inner
                                .dom_expandos
                                .entry(node.id)
                                .or_default()
                                .insert(key, val);
                            Ok(())
                        }),
                        _ => Err("cannot assign index on non-object".into()),
                    }
                }
            }
        }
        LValue::MemberExpr(base, field) => assign_expr_member(ctx, base.as_ref(), &field, val),
        LValue::IndexExpr(base, index_expr) => {
            let key = index_to_key(&eval_expr(ctx, *index_expr)?);
            if matches!(base.as_ref(), Expr::This) {
                return write_this_index(ctx, &key, val);
            }
            if let Expr::Var(name) = base.as_ref() {
                return assign_index_on_name(ctx, name, &key, val);
            }
            if let Some(LValue::Name(name)) = expr_to_lvalue(base.as_ref()) {
                return assign_index_on_name(ctx, &name, &key, val);
            }
            let parent = eval_expr(ctx, *base)?;
            if let Kv8Value::Dom(node) = parent {
                return ctx.with_mut(|inner| {
                    inner
                        .dom_expandos
                        .entry(node.id)
                        .or_default()
                        .insert(key, val);
                    Ok(())
                });
            }
            set_prop_on_value(ctx, &parent, &key, val)
        }
    }
}

fn is_global_this_value(v: &Kv8Value) -> bool {
    matches!(
        v,
        Kv8Value::Obj(m)
            if m.get("__native").and_then(|n| n.as_str()) == Some("globalThis")
    )
}

fn global_this_object(ctx: &Kv8Context) -> Result<Kv8Value, String> {
    ctx.with_mut(|inner| {
        if inner.global_this.is_none() {
            let mut m = HashMap::new();
            m.insert("__native".into(), Kv8Value::Str("globalThis".into()));
            inner.global_this = Some(Kv8Value::Obj(m));
        }
        Ok(inner.global_this.clone().unwrap())
    })
}

fn set_global_prop(ctx: &Kv8Context, field: &str, val: Kv8Value) -> Result<(), String> {
    ctx.with_mut(|inner| {
        if inner.global_this.is_none() {
            let mut m = HashMap::new();
            m.insert("__native".into(), Kv8Value::Str("globalThis".into()));
            inner.global_this = Some(Kv8Value::Obj(m));
        }
        if let Some(Kv8Value::Obj(m)) = inner.global_this.as_mut() {
            m.insert(field.to_string(), val);
            Ok(())
        } else {
            Err("globalThis corrupt".into())
        }
    })
}

fn assign_index_on_name(
    ctx: &Kv8Context,
    name: &str,
    key: &str,
    val: Kv8Value,
) -> Result<(), String> {
    ctx.with_mut(|inner| {
        let cell = inner
            .scope_resolve_mut(name)
            .ok_or_else(|| format!("cannot assign to undefined {name}"))?;
        match cell {
            Kv8Value::Obj(m) => {
                if let Some(id) = plain_obj_id(m) {
                    inner
                        .obj_store
                        .entry(id)
                        .or_default()
                        .insert(key.to_string(), val);
                } else {
                    set_index_on_obj(m, key, val);
                }
                Ok(())
            }
            Kv8Value::Dom(node) => {
                let id = node.id;
                inner
                    .dom_expandos
                    .entry(id)
                    .or_default()
                    .insert(key.to_string(), val);
                Ok(())
            }
            _ => Err(format!(
                "cannot assign index on non-object ({name}={})",
                callee_debug_hint(cell)
            )),
        }
    })
}

fn set_prop_on_value(
    ctx: &Kv8Context,
    parent: &Kv8Value,
    key: &str,
    val: Kv8Value,
) -> Result<(), String> {
    if is_global_this_value(parent) {
        return set_global_prop(ctx, key, val);
    }
    if let Kv8Value::Dom(node) = parent {
        return ctx.with_mut(|inner| {
            inner
                .dom_expandos
                .entry(node.id)
                .or_default()
                .insert(key.to_string(), val);
            Ok(())
        });
    }
    if let Kv8Value::Obj(m) = parent {
        if let Some(id) = plain_obj_id(m) {
            return obj_store_set(ctx, id, key, val);
        }
        if m.get("__native")
            .and_then(|n| n.as_str())
            .is_some_and(|n| n.starts_with("element.") || n == "document" || n == "console")
        {
            return Err("cannot assign to computed member".into());
        }
    }
    Err("cannot assign to computed member".into())
}

fn assign_expr_member(
    ctx: &Kv8Context,
    base: &Expr,
    field: &str,
    val: Kv8Value,
) -> Result<(), String> {
    if matches!(base, Expr::Var(name) if name == "globalThis" || name == "self") {
        return set_global_prop(ctx, field, val);
    }
    if let Expr::Var(name) = base {
        return ctx.with_mut(|inner| {
            let cell = inner
                .scope_resolve_mut(name)
                .ok_or_else(|| format!("cannot assign to undefined {name}"))?;
            if let Kv8Value::Obj(m) = cell {
                m.insert(field.to_string(), val);
                Ok(())
            } else {
                Err("cannot assign to member of non-object".into())
            }
        });
    }
    let parent = eval_expr(ctx, base.clone())?;
    set_prop_on_value(ctx, &parent, field, val)
}

fn write_dom_binding(ctx: &Kv8Context, base: LValue, node: DomNode) -> Result<(), String> {
    ctx.publish_node(node.clone())?;
    if let LValue::Name(var) = base {
        ctx.with_mut(|inner| {
            inner.scope_current_mut().insert(var, Kv8Value::Dom(node));
            Ok(())
        })?;
    }
    Ok(())
}

fn eval_lhs_assignments_in_lvalue(ctx: &Kv8Context, lv: &LValue) -> Result<(), String> {
    match lv {
        LValue::Member(inner, _) | LValue::Index(inner, _) => {
            eval_lhs_assignments_in_lvalue(ctx, inner)
        }
        LValue::MemberExpr(base, _) | LValue::IndexExpr(base, _) => {
            if let Expr::AssignExpr(inner_lv, op, rhs) = base.as_ref() {
                if *op == '=' {
                    let val = eval_expr(ctx, *rhs.clone())?;
                    assign_lvalue(ctx, inner_lv.clone(), val)?;
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn eval_lvalue_as_obj(ctx: &Kv8Context, lv: LValue) -> Result<Kv8Value, String> {
    match lv {
        LValue::Name(name) => {
            if let Some(v) = ctx.with_mut(|inner| Ok(inner.scope_get(&name)))? {
                return Ok(v);
            }
            eval_var(ctx, &name)
        }
        LValue::This => current_this(ctx),
        LValue::Member(base, field) => {
            let v = eval_lvalue_as_obj(ctx, *base)?;
            member_get(ctx, v, &field)
        }
        LValue::Index(base, index_expr) => {
            let parent = eval_lvalue_as_obj(ctx, *base)?;
            let idx = eval_expr(ctx, *index_expr)?;
            index_get(ctx, parent, idx)
        }
        LValue::MemberExpr(base, field) => {
            let v = eval_expr(ctx, *base)?;
            member_get(ctx, v, &field)
        }
        LValue::IndexExpr(base, index_expr) => {
            let parent = eval_expr(ctx, *base)?;
            let idx = eval_expr(ctx, *index_expr)?;
            index_get(ctx, parent, idx)
        }
    }
}

fn write_this_member(ctx: &Kv8Context, field: &str, val: Kv8Value) -> Result<(), String> {
    ctx.with_mut(|inner| {
        let this = inner
            .this_stack
            .last_mut()
            .ok_or("no this binding")?;
        match this {
            Kv8Value::Obj(m) => {
                if let Some(id) = plain_obj_id(m) {
                    inner
                        .obj_store
                        .entry(id)
                        .or_default()
                        .insert(field.to_string(), val);
                } else {
                    m.insert(field.to_string(), val);
                }
                Ok(())
            }
            Kv8Value::Dom(node) => {
                let id = node.id;
                inner
                    .dom_expandos
                    .entry(id)
                    .or_default()
                    .insert(field.to_string(), val);
                Ok(())
            }
            _ => Err("cannot set property on this".into()),
        }
    })
}

fn write_this_index(ctx: &Kv8Context, key: &str, val: Kv8Value) -> Result<(), String> {
    ctx.with_mut(|inner| {
        let this = inner
            .this_stack
            .last_mut()
            .ok_or("no this binding")?;
        match this {
            Kv8Value::Obj(m) => {
                if let Some(id) = plain_obj_id(m) {
                    inner
                        .obj_store
                        .entry(id)
                        .or_default()
                        .insert(key.to_string(), val);
                } else {
                    set_index_on_obj(m, key, val);
                }
                Ok(())
            }
            Kv8Value::Dom(node) => {
                let id = node.id;
                inner
                    .dom_expandos
                    .entry(id)
                    .or_default()
                    .insert(key.to_string(), val);
                Ok(())
            }
            _ => Err("cannot assign index on non-object".into()),
        }
    })
}

fn eval_var(ctx: &Kv8Context, name: &str) -> Result<Kv8Value, String> {
    match name {
        "Symbol" => symbol_namespace(ctx),
        "Object" => Ok(object_namespace(ctx)),
        "Array" => Ok(array_namespace()),
        "setTimeout" => Ok(global_native("setTimeout")),
        "clearTimeout" => Ok(global_native("clearTimeout")),
        "setInterval" => Ok(global_native("setInterval")),
        "clearInterval" => Ok(global_native("clearInterval")),
        "Promise" => Ok(promise_namespace_object()),
        "fetch" => Ok(global_native("fetch")),
        "localStorage" => Ok(local_storage_object()),
        "requestAnimationFrame" => Ok(global_native("requestAnimationFrame")),
        "cancelAnimationFrame" => Ok(global_native("cancelAnimationFrame")),
        "globalThis" | "self" | "window" => global_this_object(ctx),
        "undefined" => Ok(Kv8Value::Undefined),
        "Error" => Ok(global_native("Error")),
        "Map" => Ok(map_namespace()),
        "Set" => Ok(set_namespace()),
        "FormData" => Ok(form_data_namespace()),
        "Date" => Ok(date_namespace()),
        "Math" => Ok(math_namespace()),
        "String" => Ok(string_namespace()),
        "performance" => Ok(performance_object()),
        "encodeURIComponent" => Ok(global_native("encodeURIComponent")),
        "isNaN" => Ok(global_native("isNaN")),
        "RegExp" => Ok(global_native("RegExp")),
        "WeakMap" => Ok(global_native("WeakMap")),
        "document" | "console" => ctx.with_mut(|inner| {
            if name == "document" {
                if inner.opt.document_singleton.is_none() {
                    inner.opt.document_singleton = Some(document_object());
                }
                return Ok(inner.opt.document_singleton.clone().unwrap());
            }
            if inner.opt.console_singleton.is_none() {
                inner.opt.console_singleton = Some(console_object());
            }
            Ok(inner.opt.console_singleton.clone().unwrap())
        }),
        _ => {
            if let Some(v) = ctx.with_read(|inner| Ok(inner.scope_get(name)))? {
                return Ok(v);
            }
            ctx.with_mut(|inner| {
                if let Some(v) = inner.try_materialize_forward_fun(name) {
                    return Ok(v);
                }
                Ok(Kv8Value::Undefined)
            })
        }
    }
}

fn bump_eval_budget(ctx: &Kv8Context) -> Result<(), String> {
    ctx.bump_eval_ops()
}

fn eval_expr(ctx: &Kv8Context, expr: Expr) -> Result<Kv8Value, String> {
    bump_eval_budget(ctx)?;
    match expr {
        Expr::Lit(v) => Ok(v),
        Expr::Var(name) => eval_var(ctx, &name),
        Expr::This => current_this(ctx),
        Expr::FunExpr(params, body) => new_fun(ctx, params, body),
        Expr::Member(base, field) => {
            let v = eval_expr(ctx, *base)?;
            member_get(ctx, v, &field)
        }
        Expr::OptMember(base, field) => {
            let v = eval_expr(ctx, *base)?;
            if is_nullish(&v) {
                return Ok(Kv8Value::Undefined);
            }
            member_get(ctx, v, &field)
        }
        Expr::Index(base, idx) => {
            let v = eval_expr(ctx, *base)?;
            let i = eval_expr(ctx, *idx)?;
            index_get(ctx, v, i)
        }
        Expr::OptIndex(base, idx) => {
            let v = eval_expr(ctx, *base)?;
            if is_nullish(&v) {
                return Ok(Kv8Value::Undefined);
            }
            let i = eval_expr(ctx, *idx)?;
            index_get(ctx, v, i)
        }
        Expr::OptCall(callee, args) => {
            let base = eval_expr(ctx, *callee)?;
            if is_nullish(&base) {
                return Ok(Kv8Value::Undefined);
            }
            let evaluated: Result<Vec<Kv8Value>, String> =
                args.into_iter().map(|a| eval_expr(ctx, a)).collect();
            call_value(ctx, base, evaluated?)
        }
        Expr::Template(parts) => {
            let mut out = String::new();
            for part in parts {
                match part {
                    TemplatePart::Lit(s) => out.push_str(&s),
                    TemplatePart::Expr(e) => {
                        out.push_str(&kv8_value_to_display(&eval_expr(ctx, *e)?));
                    }
                }
            }
            Ok(Kv8Value::Str(out))
        }
        Expr::Call(callee, args) => eval_call(ctx, *callee, args),
        Expr::Unary(op, inner) => {
            if op == 'd' {
                if let Some(lv) = expr_to_lvalue(&inner) {
                    return Ok(Kv8Value::Bool(delete_lvalue(ctx, &lv)?));
                }
                return Ok(Kv8Value::Bool(true));
            }
            let v = eval_expr(ctx, *inner)?;
            match op {
                '!' => Ok(Kv8Value::Bool(!v.is_truthy())),
                '-' => match v {
                    Kv8Value::Num(n) => Ok(Kv8Value::Num(-n)),
                    _ => Err("unary - expects number".into()),
                },
                '+' => match v {
                    Kv8Value::Num(n) => Ok(Kv8Value::Num(n)),
                    _ => Err("unary + expects number".into()),
                },
                't' => Ok(Kv8Value::Str(kv8_typeof(&v))),
                'v' => Ok(Kv8Value::Undefined),
                '~' => Ok(Kv8Value::Num((!to_int32(&v)) as f64)),
                _ => Err("unsupported unary".into()),
            }
        }
        Expr::Seq(exprs) => {
            let mut last = Kv8Value::Undefined;
            for e in exprs {
                last = eval_expr(ctx, e)?;
            }
            Ok(last)
        }
        Expr::AssignExpr(lv, op, rhs) => {
            let val = if op == '=' {
                eval_lhs_assignments_in_lvalue(ctx, &lv)?;
                eval_expr(ctx, *rhs)?
            } else {
                let cur = read_lvalue(ctx, &lv)?;
                let r = eval_expr(ctx, *rhs)?;
                eval_compound(op, &cur, &r)?
            };
            assign_lvalue(ctx, lv, val.clone())?;
            Ok(val)
        }
        Expr::Cond(cond, then_e, else_e) => {
            if eval_expr(ctx, *cond)?.is_truthy() {
                eval_expr(ctx, *then_e)
            } else {
                eval_expr(ctx, *else_e)
            }
        }
        Expr::Update(lv, op, prefix) => {
            let cur = read_lvalue(ctx, &lv)?;
            let cur_n = cur.as_num().unwrap_or(0.0);
            let new_v = if op == '+' {
                Kv8Value::Num(cur_n + 1.0)
            } else {
                Kv8Value::Num(cur_n - 1.0)
            };
            assign_lvalue(ctx, lv, new_v.clone())?;
            Ok(if prefix { new_v } else { cur })
        }
        Expr::Bin(l, op, r) => {
            let a = eval_expr(ctx, *l)?;
            match op {
                '&' => {
                    if !a.is_truthy() {
                        return Ok(a);
                    }
                    eval_expr(ctx, *r)
                }
                '|' => {
                    if a.is_truthy() {
                        return Ok(a);
                    }
                    eval_expr(ctx, *r)
                }
                'n' => {
                    if is_nullish(&a) {
                        eval_expr(ctx, *r)
                    } else {
                        Ok(a)
                    }
                }
                _ => {
                    let b = eval_expr(ctx, *r)?;
                    Ok(eval_bin(op, &a, &b))
                }
            }
        }
        Expr::Arrow(params, body) => new_arrow(ctx, params, body),
        Expr::Block(stmts) => flow_to_value(run_stmts(ctx, &stmts)?),
        Expr::Object(pairs) => {
            let mut map = HashMap::new();
            for (k, e) in pairs {
                match k {
                    ObjectEntryKey::Spread(expr) => {
                        if let Kv8Value::Obj(m) = eval_expr(ctx, expr)? {
                            for (pk, pv) in m {
                                map.insert(pk, pv);
                            }
                        }
                        continue;
                    }
                    ObjectEntryKey::Lit(s) => {
                        map.insert(s, eval_expr(ctx, e)?);
                    }
                    ObjectEntryKey::Computed(expr) => {
                        let key = index_to_key(&eval_expr(ctx, expr)?);
                        map.insert(key, eval_expr(ctx, e)?);
                    }
                }
            }
            attach_object_prototype(&mut map);
            register_plain_obj(ctx, &mut map);
            Ok(Kv8Value::Obj(map))
        }
        Expr::Array(elems) => {
            let values: Result<Vec<Kv8Value>, String> =
                elems.into_iter().map(|e| eval_expr(ctx, e)).collect();
            Ok(array_from_values(values?))
        }
        Expr::New(callee, args) => {
            let site = match callee.as_ref() {
                Expr::Var(name) => name.clone(),
                Expr::Member(_, field) => format!(".{field}"),
                _ => "new".into(),
            };
            let evaluated: Result<Vec<Kv8Value>, String> =
                args.into_iter().map(|a| eval_expr(ctx, a)).collect();
            let ctor = eval_expr(ctx, *callee)?;
            if matches!(ctor, Kv8Value::Undefined) {
                return Err(format_call_error(
                    ctx,
                    format!("unknown constructor: undefined at {site}"),
                ));
            }
            construct_new(ctx, ctor, evaluated?)
        }
        Expr::Await(inner) => {
            let depth = ctx.with_mut(|inner| Ok(inner.in_async))?;
            if depth == 0 {
                return Err("await is only valid inside async functions".into());
            }
            let v = eval_expr(ctx, *inner)?;
            await_value(ctx, v)
        }
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

fn kv8_typeof(v: &Kv8Value) -> String {
    if is_kv8_callable(v) {
        return "function".into();
    }
    match v {
        Kv8Value::Undefined => "undefined".into(),
        Kv8Value::Null => "object".into(),
        Kv8Value::Bool(_) => "boolean".into(),
        Kv8Value::Num(_) => "number".into(),
        Kv8Value::Str(_) => "string".into(),
        Kv8Value::Dom(_) | Kv8Value::Obj(_) | Kv8Value::Promise(_) => "object".into(),
        Kv8Value::Symbol { .. } => "symbol".into(),
        Kv8Value::Fun { .. } | Kv8Value::Arrow { .. } | Kv8Value::AsyncFun { .. } => {
            "function".into()
        }
    }
}

fn kv8_strict_eq(a: &Kv8Value, b: &Kv8Value) -> bool {
    match (a, b) {
        (Kv8Value::Undefined, Kv8Value::Undefined) => true,
        (Kv8Value::Null, Kv8Value::Null) => true,
        (Kv8Value::Bool(x), Kv8Value::Bool(y)) => x == y,
        (Kv8Value::Num(x), Kv8Value::Num(y)) => x == y,
        (Kv8Value::Str(x), Kv8Value::Str(y)) => x == y,
        (Kv8Value::Dom(x), Kv8Value::Dom(y)) => x.id == y.id,
        (Kv8Value::Obj(x), Kv8Value::Obj(y)) => kv8_obj_eq(x, y),
        (Kv8Value::Promise(x), Kv8Value::Promise(y)) => std::rc::Rc::ptr_eq(x, y),
        (Kv8Value::Symbol { id: x, .. }, Kv8Value::Symbol { id: y, .. }) => x == y,
        _ => false,
    }
}

fn kv8_obj_eq(a: &HashMap<String, Kv8Value>, b: &HashMap<String, Kv8Value>) -> bool {
    a.len() == b.len()
        && a.iter()
            .all(|(k, v)| b.get(k).is_some_and(|bv| kv8_strict_eq(v, bv)))
}

fn kv8_loose_eq(a: &Kv8Value, b: &Kv8Value) -> bool {
    if kv8_strict_eq(a, b) {
        return true;
    }
    match (a, b) {
        (Kv8Value::Null, Kv8Value::Undefined) | (Kv8Value::Undefined, Kv8Value::Null) => true,
        (Kv8Value::Num(x), Kv8Value::Str(y)) | (Kv8Value::Str(y), Kv8Value::Num(x)) => y
            .parse::<f64>()
            .ok()
            .is_some_and(|n| n == *x),
        (Kv8Value::Bool(bv), Kv8Value::Num(n)) | (Kv8Value::Num(n), Kv8Value::Bool(bv)) => {
            (*bv && *n == 1.0) || (!bv && *n == 0.0)
        }
        _ => false,
    }
}

fn to_int32(v: &Kv8Value) -> i32 {
    let n = v.as_num().unwrap_or(0.0);
    if !n.is_finite() || n == 0.0 {
        return 0;
    }
    let mut n = n.trunc().rem_euclid(4294967296.0);
    if n >= 2147483648.0 {
        n -= 4294967296.0;
    }
    n as i32
}

fn to_uint32(v: &Kv8Value) -> u32 {
    to_int32(v) as u32
}

fn kv8_in_operator(key: &Kv8Value, obj: &Kv8Value) -> bool {
    let key_s = match key {
        Kv8Value::Str(s) => s.clone(),
        Kv8Value::Num(n) => {
            if n.fract() == 0.0 && *n >= 0.0 {
                format!("{}", *n as i64)
            } else {
                n.to_string()
            }
        }
        _ => return false,
    };
    match obj {
        Kv8Value::Obj(m) => m.contains_key(&key_s),
        _ => false,
    }
}

fn kv8_instanceof(val: &Kv8Value, ctor: &Kv8Value) -> bool {
    let Some(ctor_name) = ctor_name_of(ctor) else {
        return false;
    };
    match val {
        Kv8Value::Obj(m) => m
            .get("__native")
            .and_then(|v| v.as_str())
            .is_some_and(|tag| tag.eq_ignore_ascii_case(&ctor_name) || ctor_name.ends_with(tag)),
        _ => false,
    }
}

fn ctor_name_of(ctor: &Kv8Value) -> Option<String> {
    match ctor {
        Kv8Value::Obj(m) => m
            .get("__ctor")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| {
                m.get("__native")
                    .and_then(|v| v.as_str())
                    .map(String::from)
            }),
        Kv8Value::Fun { .. } | Kv8Value::AsyncFun { .. } => Some("Function".into()),
        _ => None,
    }
}

fn kv8_value_to_display(v: &Kv8Value) -> String {
    match v {
        Kv8Value::Str(s) => s.clone(),
        Kv8Value::Num(n) => {
            if n.fract() == 0.0 {
                format!("{}", *n as i64)
            } else {
                n.to_string()
            }
        }
        Kv8Value::Bool(b) => b.to_string(),
        Kv8Value::Null => "null".into(),
        Kv8Value::Undefined => "undefined".into(),
        _ => String::new(),
    }
}

fn eval_bin(op: char, a: &Kv8Value, b: &Kv8Value) -> Kv8Value {
    match op {
        '+' => {
            if matches!(a, Kv8Value::Str(_)) || matches!(b, Kv8Value::Str(_)) {
                Kv8Value::Str(format!(
                    "{}{}",
                    kv8_value_to_display(a),
                    kv8_value_to_display(b)
                ))
            } else {
                Kv8Value::Num(a.as_num().unwrap_or(0.0) + b.as_num().unwrap_or(0.0))
            }
        }
        '-' => Kv8Value::Num(a.as_num().unwrap_or(0.0) - b.as_num().unwrap_or(0.0)),
        '*' => Kv8Value::Num(a.as_num().unwrap_or(0.0) * b.as_num().unwrap_or(0.0)),
        '/' => Kv8Value::Num(a.as_num().unwrap_or(0.0) / b.as_num().unwrap_or(1.0)),
        '%' => {
            let denom = b.as_num().unwrap_or(1.0);
            Kv8Value::Num(a.as_num().unwrap_or(0.0) % denom)
        }
        '=' => Kv8Value::Bool(kv8_loose_eq(a, b)),
        '!' => Kv8Value::Bool(!kv8_loose_eq(a, b)),
        'E' => Kv8Value::Bool(kv8_strict_eq(a, b)),
        'e' => Kv8Value::Bool(!kv8_strict_eq(a, b)),
        '<' => Kv8Value::Bool(a.as_num().unwrap_or(0.0) < b.as_num().unwrap_or(0.0)),
        'l' => Kv8Value::Bool(a.as_num().unwrap_or(0.0) <= b.as_num().unwrap_or(0.0)),
        '>' => Kv8Value::Bool(a.as_num().unwrap_or(0.0) > b.as_num().unwrap_or(0.0)),
        'g' => Kv8Value::Bool(a.as_num().unwrap_or(0.0) >= b.as_num().unwrap_or(0.0)),
        '&' => Kv8Value::Bool(a.is_truthy() && b.is_truthy()),
        '|' => Kv8Value::Bool(a.is_truthy() || b.is_truthy()),
        'A' => Kv8Value::Num((to_int32(a) & to_int32(b)) as f64),
        'o' => Kv8Value::Num((to_int32(a) | to_int32(b)) as f64),
        '^' => Kv8Value::Num((to_int32(a) ^ to_int32(b)) as f64),
        'L' => Kv8Value::Num((to_int32(a) << (to_uint32(b) & 0x1f)) as f64),
        'R' => Kv8Value::Num((to_int32(a) >> (to_uint32(b) & 0x1f)) as f64),
        'U' => Kv8Value::Num((to_uint32(a) >> (to_uint32(b) & 0x1f)) as f64),
        'i' => Kv8Value::Bool(kv8_in_operator(a, b)),
        'I' => Kv8Value::Bool(kv8_instanceof(a, b)),
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

fn global_native(name: &str) -> Kv8Value {
    let mut m = HashMap::new();
    m.insert("__native".into(), Kv8Value::Str(name.into()));
    Kv8Value::Obj(m)
}

fn local_storage_object() -> Kv8Value {
    let mut m = HashMap::new();
    m.insert("__native".into(), Kv8Value::Str("localStorage".into()));
    Kv8Value::Obj(m)
}

fn local_storage_method(name: &str) -> Kv8Value {
    let mut m = HashMap::new();
    m.insert(
        "__native".into(),
        Kv8Value::Str(format!("localStorage.{name}")),
    );
    Kv8Value::Obj(m)
}

fn promise_namespace_object() -> Kv8Value {
    let mut m = HashMap::new();
    m.insert("__native".into(), Kv8Value::Str("Promise".into()));
    Kv8Value::Obj(m)
}

fn array_namespace() -> Kv8Value {
    let mut is_array = HashMap::new();
    is_array.insert("__native".into(), Kv8Value::Str("Array.isArray".into()));
    let mut m = HashMap::new();
    m.insert("__native".into(), Kv8Value::Str("Array".into()));
    m.insert("isArray".into(), Kv8Value::Obj(is_array));
    Kv8Value::Obj(m)
}

fn object_prototype_value() -> Kv8Value {
    let mut has_own = HashMap::new();
    has_own.insert(
        "__native".into(),
        Kv8Value::Str("Object.prototype.hasOwnProperty".into()),
    );
    let mut proto = HashMap::new();
    proto.insert("__native".into(), Kv8Value::Str("Object.prototype".into()));
    proto.insert("hasOwnProperty".into(), Kv8Value::Obj(has_own));
    Kv8Value::Obj(proto)
}

fn register_plain_obj(ctx: &Kv8Context, map: &mut HashMap<String, Kv8Value>) {
    let _ = ctx.with_mut(|inner| {
        let id = inner.next_obj_id;
        inner.next_obj_id += 1;
        map.insert("__obj_id".into(), Kv8Value::Num(id as f64));
        inner.obj_store.insert(id, HashMap::new());
        Ok(())
    });
}

fn plain_obj_id(map: &HashMap<String, Kv8Value>) -> Option<u64> {
    map.get("__obj_id")
        .and_then(|v| v.as_num())
        .map(|n| n as u64)
}

fn obj_store_get(ctx: &Kv8Context, id: u64, key: &str) -> Result<Kv8Value, String> {
    ctx.with_mut(|inner| {
        Ok(inner
            .obj_store
            .get(&id)
            .and_then(|m| m.get(key))
            .cloned()
            .unwrap_or(Kv8Value::Undefined))
    })
}

fn obj_store_set(ctx: &Kv8Context, id: u64, key: &str, val: Kv8Value) -> Result<(), String> {
    ctx.with_mut(|inner| {
        inner
            .obj_store
            .entry(id)
            .or_default()
            .insert(key.to_string(), val);
        Ok(())
    })
}

fn object_desc_store_key(prop: &str) -> String {
    format!("__desc__:{prop}")
}

fn is_object_own_key(key: &str) -> bool {
    !key.starts_with("__")
}

fn object_static_method(name: &str) -> Kv8Value {
    global_native(name)
}

fn object_descriptor_get(
    ctx: &Kv8Context,
    map: &HashMap<String, Kv8Value>,
    field: &str,
) -> Result<Option<Kv8Value>, String> {
    if let Some(id) = plain_obj_id(map) {
        if let Ok(Kv8Value::Obj(desc)) =
            obj_store_get(ctx, id, &object_desc_store_key(field))
        {
            return Ok(Some(Kv8Value::Obj(desc)));
        }
    }
    if let Some(Kv8Value::Obj(descs)) = map.get("__descs__") {
        if let Some(desc) = descs.get(field) {
            return Ok(Some(desc.clone()));
        }
    }
    Ok(None)
}

fn object_property_get(
    ctx: &Kv8Context,
    map: &HashMap<String, Kv8Value>,
    field: &str,
) -> Result<Option<Kv8Value>, String> {
    if let Some(Kv8Value::Obj(desc)) = object_descriptor_get(ctx, map, field)? {
        if let Some(getter) = desc.get("get") {
            if !matches!(getter, Kv8Value::Undefined) {
                let this_obj = Kv8Value::Obj(map.clone());
                return Ok(Some(call_value_with_this(
                    ctx,
                    getter.clone(),
                    vec![],
                    Some(this_obj),
                )?));
            }
        }
        if let Some(value) = desc.get("value") {
            return Ok(Some(value.clone()));
        }
        return Ok(Some(Kv8Value::Undefined));
    }
    if let Some(id) = plain_obj_id(map) {
        let v = obj_store_get(ctx, id, field)?;
        if !matches!(v, Kv8Value::Undefined) {
            return Ok(Some(v));
        }
    }
    if let Some(v) = map.get(field) {
        return Ok(Some(v.clone()));
    }
    Ok(None)
}

fn object_store_descriptor(
    ctx: &Kv8Context,
    map: &HashMap<String, Kv8Value>,
    key: &str,
    desc: HashMap<String, Kv8Value>,
) -> Result<(), String> {
    if let Some(id) = plain_obj_id(map) {
        obj_store_set(ctx, id, &object_desc_store_key(key), Kv8Value::Obj(desc))?;
        return Ok(());
    }
    Err("Object.defineProperty: target has no object id".into())
}

fn object_define_property(
    ctx: &Kv8Context,
    target: Kv8Value,
    key: &str,
    desc: Kv8Value,
) -> Result<Kv8Value, String> {
    let Kv8Value::Obj(map) = target else {
        return Err("Object.defineProperty: non-object target".into());
    };
    let Kv8Value::Obj(desc_map) = desc else {
        return Err("Object.defineProperty: descriptor must be object".into());
    };
    let has_getter = desc_map
        .get("get")
        .map(|g| !matches!(g, Kv8Value::Undefined))
        .unwrap_or(false);
    let has_setter = desc_map
        .get("set")
        .map(|s| !matches!(s, Kv8Value::Undefined))
        .unwrap_or(false);
    object_store_descriptor(ctx, &map, key, desc_map.clone())?;
    if let Some(id) = plain_obj_id(&map) {
        if has_getter || has_setter {
            obj_store_set(ctx, id, key, Kv8Value::Undefined)?;
        } else if let Some(value) = desc_map.get("value") {
            obj_store_set(ctx, id, key, value.clone())?;
        }
    }
    Ok(Kv8Value::Obj(map))
}

fn object_create(ctx: &Kv8Context, proto: Kv8Value) -> Result<Kv8Value, String> {
    let mut map = HashMap::new();
    match proto {
        Kv8Value::Null => {}
        Kv8Value::Undefined => attach_object_prototype(&mut map),
        p => {
            map.insert("__proto__".into(), p);
        }
    }
    register_plain_obj(ctx, &mut map);
    Ok(Kv8Value::Obj(map))
}

fn object_get_prototype_of(obj: Kv8Value) -> Result<Kv8Value, String> {
    let Kv8Value::Obj(map) = obj else {
        return Err("Object.getPrototypeOf: non-object".into());
    };
    Ok(map
        .get("__proto__")
        .cloned()
        .unwrap_or(Kv8Value::Null))
}

fn object_get_own_property_descriptor(
    ctx: &Kv8Context,
    obj: Kv8Value,
    key: &str,
) -> Result<Kv8Value, String> {
    let Kv8Value::Obj(map) = obj else {
        return Err("Object.getOwnPropertyDescriptor: non-object".into());
    };
    if let Some(desc) = object_descriptor_get(ctx, &map, key)? {
        return Ok(desc);
    }
    let raw = if let Some(id) = plain_obj_id(&map) {
        obj_store_get(ctx, id, key)?
    } else {
        map.get(key).cloned().unwrap_or(Kv8Value::Undefined)
    };
    if matches!(raw, Kv8Value::Undefined) && !map.contains_key(key) {
        return Ok(Kv8Value::Undefined);
    }
    let mut desc = HashMap::new();
    desc.insert("value".into(), raw);
    desc.insert("writable".into(), Kv8Value::Bool(true));
    desc.insert("enumerable".into(), Kv8Value::Bool(true));
    desc.insert("configurable".into(), Kv8Value::Bool(true));
    Ok(Kv8Value::Obj(desc))
}

fn object_own_property_names(
    ctx: &Kv8Context,
    obj: Kv8Value,
) -> Result<Kv8Value, String> {
    let Kv8Value::Obj(map) = obj else {
        return Err("Object.getOwnPropertyNames: non-object".into());
    };
    let mut keys = HashSet::new();
    for k in map.keys() {
        if is_object_own_key(k) {
            keys.insert(k.clone());
        }
    }
    if let Some(id) = plain_obj_id(&map) {
        ctx.with_mut(|inner| {
            if let Some(store) = inner.obj_store.get(&id) {
                for k in store.keys() {
                    if let Some(prop) = k.strip_prefix("__desc__:") {
                        keys.insert(prop.to_string());
                    } else if is_object_own_key(k) {
                        keys.insert(k.clone());
                    }
                }
            }
            Ok(())
        })?;
    }
    let mut names: Vec<String> = keys.into_iter().collect();
    names.sort();
    Ok(array_from_values(
        names.into_iter().map(Kv8Value::Str).collect(),
    ))
}

fn attach_object_prototype(map: &mut HashMap<String, Kv8Value>) {
    if map.contains_key("__proto__") || map.get("__native").is_some() {
        return;
    }
    map.insert("__proto__".into(), object_prototype_value());
}

fn object_namespace(ctx: &Kv8Context) -> Kv8Value {
    let mut m = HashMap::new();
    m.insert("__native".into(), Kv8Value::Str("Object".into()));
    m.insert("prototype".into(), object_prototype_value());
    m.insert("assign".into(), object_static_method("Object.assign"));
    m.insert("create".into(), object_static_method("Object.create"));
    m.insert(
        "defineProperty".into(),
        object_static_method("Object.defineProperty"),
    );
    m.insert(
        "getOwnPropertyDescriptor".into(),
        object_static_method("Object.getOwnPropertyDescriptor"),
    );
    m.insert(
        "getOwnPropertyNames".into(),
        object_static_method("Object.getOwnPropertyNames"),
    );
    m.insert(
        "getPrototypeOf".into(),
        object_static_method("Object.getPrototypeOf"),
    );
    let _ = ctx;
    Kv8Value::Obj(m)
}

fn symbol_namespace(ctx: &Kv8Context) -> Result<Kv8Value, String> {
    let iterator = ctx.well_known_symbol("Symbol.iterator")?;
    let mut sym_for = HashMap::new();
    sym_for.insert("__native".into(), Kv8Value::Str("Symbol.for".into()));
    let mut m = HashMap::new();
    m.insert("__native".into(), Kv8Value::Str("Symbol".into()));
    m.insert("for".into(), Kv8Value::Obj(sym_for));
    m.insert("iterator".into(), iterator);
    Ok(Kv8Value::Obj(m))
}

fn promise_static_method(name: &str) -> Kv8Value {
    global_native(&format!("Promise.{name}"))
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
                if field == "body" {
                    return Ok(Kv8Value::Dom(ctx.body_node()?));
                }
                return Ok(document_method(field));
            }
            if map.get("__native").and_then(|v| v.as_str()) == Some("console") {
                return Ok(console_method(field));
            }
            if map.get("__native").and_then(|v| v.as_str()) == Some("Promise") {
                return Ok(promise_static_method(field));
            }
            if map.get("__native").and_then(|v| v.as_str()) == Some("localStorage") {
                return Ok(local_storage_method(field));
            }
            if map.get("__native").and_then(|v| v.as_str()) == Some("NodeList") {
                if field == "forEach" {
                    let list_id = map
                        .get("__nodelist_id")
                        .and_then(|v| v.as_num())
                        .unwrap_or(0.0) as u64;
                    return Ok(node_list_method("forEach", list_id));
                }
            }
            if map.get("__native").and_then(|v| v.as_str()) == Some("Array") {
                if field == "length" {
                    return Ok(map
                        .get("length")
                        .cloned()
                        .unwrap_or(Kv8Value::Num(0.0)));
                }
                match field {
                    "push" | "pop" | "shift" | "unshift" | "splice" | "slice" | "concat"
                    | "forEach" | "map" | "join" => return Ok(array_method(field)),
                    _ => {}
                }
                return Ok(map.get(field).cloned().unwrap_or(Kv8Value::Undefined));
            }
            if map.get("__native").and_then(|v| v.as_str()) == Some("Map")
                || map.get("__native").and_then(|v| v.as_str()) == Some("WeakMap")
            {
                if field == "size" {
                    let count = map
                        .iter()
                        .filter(|(k, _)| is_map_entry_key(k))
                        .count();
                    return Ok(Kv8Value::Num(count as f64));
                }
                if let Some(id) = plain_obj_id(&map) {
                    if let Ok(v) = obj_store_get(ctx, id, field) {
                        if !matches!(v, Kv8Value::Undefined) {
                            return Ok(v);
                        }
                    }
                }
                if let Some(v) = map.get(field) {
                    return Ok(v.clone());
                }
                if let Some(Kv8Value::Obj(proto)) = map.get("__proto__") {
                    if let Some(v) = proto.get(field) {
                        return Ok(v.clone());
                    }
                }
                return Ok(map_method(field));
            }
            if map.get("__native").and_then(|v| v.as_str()) == Some("Set") {
                if field == "size" {
                    let count = map
                        .iter()
                        .filter(|(k, _)| k.starts_with('\x01'))
                        .count();
                    return Ok(Kv8Value::Num(count as f64));
                }
                return Ok(set_method(field));
            }
            if map.get("__native").and_then(|v| v.as_str()) == Some("RegExp") {
                if field == "test" {
                    return Ok(regexp_method("test"));
                }
            }
            if let Some(id) = plain_obj_id(&map) {
                if let Ok(Some(v)) = object_property_get(ctx, &map, field) {
                    return Ok(v);
                }
                let _ = id;
            }
            Ok(map.get(field).cloned().unwrap_or_else(|| {
                map.get("__proto__")
                    .and_then(|p| p.as_obj())
                    .and_then(|proto| proto.get(field))
                    .cloned()
                    .or_else(|| {
                        if map.get("__native").is_some() {
                            None
                        } else {
                            object_prototype_value()
                                .as_obj()
                                .and_then(|proto| proto.get(field))
                                .cloned()
                        }
                    })
                    .unwrap_or(Kv8Value::Undefined)
            }))
        }
        Kv8Value::Promise(p) => match field {
            "then" => Ok(promise_method("then", p)),
            "catch" => Ok(promise_method("catch", p)),
            _ => Ok(Kv8Value::Undefined),
        },
        Kv8Value::Fun { prototype, .. } | Kv8Value::AsyncFun { prototype, .. } => {
            if field == "prototype" {
                Ok(Kv8Value::Obj(prototype.clone()))
            } else {
                Ok(Kv8Value::Undefined)
            }
        }
        Kv8Value::Num(_) => match field {
            "toString" => Ok(number_method("toString")),
            _ => Ok(Kv8Value::Undefined),
        },
        Kv8Value::Str(_) => match field {
            "slice" | "indexOf" | "includes" | "split" | "replace" | "trim" | "toLowerCase"
            | "toUpperCase" | "charCodeAt" | "match" => Ok(string_method(field)),
            _ => Ok(Kv8Value::Undefined),
        },
        Kv8Value::Dom(node) => match field {
            "nodeType" => Ok(Kv8Value::Num(match node.tag.as_str() {
                "#document" | "document" => 9.0,
                "#fragment" => 11.0,
                _ => 1.0,
            })),
            "ownerDocument" => ctx.owner_document_value(),
            "getRootNode" => Ok(element_method("getRootNode", node.id)),
            "id" => Ok(Kv8Value::Str(
                node.get_attr("id").unwrap_or("").to_string(),
            )),
            "tagName" => Ok(Kv8Value::Str(node.tag.clone())),
            "style" => Ok(style_object()),
            "textContent" => Ok(Kv8Value::Str(node.text.clone().unwrap_or_default())),
            "innerHTML" => Ok(Kv8Value::Str(ctx.inner_html(node.id)?)),
            "getAttribute" => Ok(element_method("getAttribute", node.id)),
            "setAttribute" => Ok(element_method("setAttribute", node.id)),
            "querySelectorAll" => Ok(element_method("querySelectorAll", node.id)),
            "appendChild" => Ok(element_method("appendChild", node.id)),
            "removeChild" => Ok(element_method("removeChild", node.id)),
            "firstChild" => match ctx.first_child(node.id)? {
                Some(c) => Ok(Kv8Value::Dom(c)),
                None => Ok(Kv8Value::Null),
            },
            "addEventListener" => Ok(element_method("addEventListener", node.id)),
            "dispatchEvent" => Ok(element_method("dispatchEvent", node.id)),
            _ => ctx.with_mut(|inner| {
                Ok(inner
                    .dom_expandos
                    .get(&node.id)
                    .and_then(|m| m.get(field))
                    .cloned()
                    .unwrap_or(Kv8Value::Undefined))
            }),
        },
        _ => Ok(Kv8Value::Undefined),
    }
}

fn style_object() -> Kv8Value {
    let mut style = HashMap::new();
    style.insert("__native".into(), Kv8Value::Str("style".into()));
    Kv8Value::Obj(style)
}

fn promise_method(name: &str, promise: super::promise::SharedKv8Promise) -> Kv8Value {
    let mut m = HashMap::new();
    m.insert("__native".into(), Kv8Value::Str(format!("promise.{name}")));
    m.insert("__promise".into(), Kv8Value::Promise(promise));
    Kv8Value::Obj(m)
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

fn node_list_object(ctx: &Kv8Context, nodes: Vec<DomNode>) -> Result<Kv8Value, String> {
    let len = nodes.len();
    let id = ctx.store_nodelist(nodes)?;
    let mut m = HashMap::new();
    m.insert("__native".into(), Kv8Value::Str("NodeList".into()));
    m.insert("__nodelist_id".into(), Kv8Value::Num(id as f64));
    m.insert("length".into(), Kv8Value::Num(len as f64));
    Ok(Kv8Value::Obj(m))
}

fn node_list_method(name: &str, list_id: u64) -> Kv8Value {
    let mut m = HashMap::new();
    m.insert(
        "__native".into(),
        Kv8Value::Str(format!("NodeList.{name}")),
    );
    m.insert("__nodelist_id".into(), Kv8Value::Num(list_id as f64));
    Kv8Value::Obj(m)
}

fn call_value(ctx: &Kv8Context, callee: Kv8Value, args: Vec<Kv8Value>) -> Result<Kv8Value, String> {
    call_value_with_this(ctx, callee, args, None)
}

fn format_call_error(ctx: &Kv8Context, msg: String) -> String {
    if msg.contains("[trace:") {
        return msg;
    }
    let trace = ctx
        .with_mut(|inner| Ok(inner.call_trace.join(" <- ")))
        .unwrap_or_default();
    if trace.is_empty() {
        msg
    } else {
        format!("{msg} [trace: {trace}]")
    }
}

fn hoist_function_vars(ctx: &Kv8Context, body: &[Stmt]) -> Result<(), String> {
    let mut names = HashSet::new();
    opt::collect_var_hoists(body, &mut names);
    if names.is_empty() {
        return Ok(());
    }
    ctx.with_mut(|inner| {
        let frame = inner.scope_current_mut();
        for name in names {
            frame.entry(name).or_insert(Kv8Value::Undefined);
        }
        Ok(())
    })
}

fn call_value_with_this(
    ctx: &Kv8Context,
    callee: Kv8Value,
    args: Vec<Kv8Value>,
    this_receiver: Option<Kv8Value>,
) -> Result<Kv8Value, String> {
    let this_val = this_receiver.unwrap_or(Kv8Value::Undefined);
    let isolate_scope = matches!(
        &callee,
        Kv8Value::Fun { .. } | Kv8Value::AsyncFun { .. }
    );
    if isolate_scope {
        ctx.with_mut(|inner| {
            inner.scope_push();
            Ok(())
        })?;
        if let Kv8Value::Fun { body, .. } | Kv8Value::AsyncFun { body, .. } = &callee {
            hoist_function_vars(ctx, body)?;
        }
        match &callee {
            Kv8Value::Fun { closure, .. } | Kv8Value::AsyncFun { closure, .. } => {
                bind_closure_env(ctx, &closure)?;
            }
            _ => {}
        }
    }
    push_this(ctx, this_val)?;
    let result = call_value_inner(ctx, callee, args);
    pop_this(ctx)?;
    if isolate_scope {
        ctx.with_mut(|inner| {
            inner.scope_pop_preserve();
            Ok(())
        })?;
    }
    result
}

fn bind_function_params(
    ctx: &Kv8Context,
    params: &[Kv8Param],
    args: &[Kv8Value],
) -> Result<(), String> {
    ctx.with_mut(|inner| {
        inner
            .scope_current_mut()
            .insert("arguments".into(), array_from_values(args.to_vec()));
        Ok(())
    })?;
    for (i, (name, default)) in params.iter().enumerate() {
        let v = if let Some(arg) = args.get(i) {
            arg.clone()
        } else if let Some(def_expr) = default {
            eval_expr(ctx, def_expr.clone())?
        } else {
            Kv8Value::Undefined
        };
        ctx.with_mut(|inner| {
            inner.scope_current_mut().insert(name.clone(), v);
            Ok(())
        })?;
    }
    Ok(())
}

fn callee_debug_hint(v: &Kv8Value) -> String {
    match v {
        Kv8Value::Undefined => "undefined".into(),
        Kv8Value::Null => "null".into(),
        Kv8Value::Bool(b) => format!("bool({b})"),
        Kv8Value::Num(n) => format!("num({n})"),
        Kv8Value::Str(s) => format!("str({})", &s[..s.len().min(40)]),
        Kv8Value::Obj(m) => {
            if let Some(n) = m.get("__native").and_then(|x| x.as_str()) {
                return format!("native({n})");
            }
            let keys: Vec<_> = m.keys().filter(|k| !k.starts_with("__")).take(8).collect();
            format!("obj{{{}}}", keys.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(","))
        }
        Kv8Value::Fun { .. } => "fun".into(),
        Kv8Value::Arrow { .. } => "arrow".into(),
        Kv8Value::AsyncFun { .. } => "asyncfun".into(),
        Kv8Value::Promise(_) => "promise".into(),
        Kv8Value::Symbol { key, .. } => format!("symbol({key})"),
        Kv8Value::Dom(n) => format!("dom({})", n.tag),
    }
}

fn call_value_inner(ctx: &Kv8Context, callee: Kv8Value, args: Vec<Kv8Value>) -> Result<Kv8Value, String> {
    match callee {
        Kv8Value::Fun { params, body, .. } => {
            bind_function_params(ctx, &params, &args)?;
            flow_fn_result(run_stmts(ctx, &body)?)
        }
        Kv8Value::AsyncFun { params, body, .. } => {
            bind_function_params(ctx, &params, &args)?;
            ctx.with_mut(|inner| {
                inner.in_async += 1;
                Ok(())
            })?;
            let promise = new_pending_promise();
            let result = flow_fn_result(run_stmts(ctx, &body)?);
            ctx.with_mut(|inner| {
                inner.in_async = inner.in_async.saturating_sub(1);
                Ok(())
            })?;
            match result {
                Ok(v) => {
                    kv8_settle_fulfilled(ctx, &promise, v)?;
                }
                Err(e) => {
                    kv8_settle_rejected(ctx, &promise, e)?;
                }
            }
            Ok(Kv8Value::Promise(promise))
        }
        Kv8Value::Arrow { params, body, closure } => {
            ctx.with_mut(|inner| {
                inner.scope_push();
                Ok(())
            })?;
            bind_closure_env(ctx, &closure)?;
            bind_function_params(ctx, &params, &args)?;
            let result = match body.as_ref() {
                Expr::Block(stmts) => flow_fn_result(run_stmts(ctx, stmts)?),
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
                    for ((p, _), v) in params.iter().zip(kabootar_args.iter()) {
                        env.set(p.clone(), v.clone());
                    }
                    let out = run_kv8_bytecode_fn(&compiled, kabootar_args, &mut env)?;
                    Ok(kabootar_to_kv8(out))
                }
            };
            ctx.with_mut(|inner| {
                inner.scope_pop();
                Ok(())
            })?;
            result
        }
        Kv8Value::Obj(_) => call_native(ctx, callee, args),
        other => Err(format!("value is not callable: {}", callee_debug_hint(&other))),
    }
}

fn call_native(ctx: &Kv8Context, callee: Kv8Value, args: Vec<Kv8Value>) -> Result<Kv8Value, String> {
    let Kv8Value::Obj(m) = callee else {
        return Err("not native".into());
    };
    let native = m.get("__native").and_then(|v| v.as_str()).map(str::to_string);
    let Some(native) = native else {
        return Err(format!(
            "value is not callable: {}",
            callee_debug_hint(&Kv8Value::Obj(m))
        ));
    };
    ctx.with_mut(|inner| {
        inner.opt.predictor.record_call(&native);
        Ok(())
    })?;
    match native.as_str() {
        "bound.call" => {
            let target = m
                .get("__target")
                .cloned()
                .ok_or("bound.call: missing target")?;
            let this_val = m
                .get("__this")
                .cloned()
                .unwrap_or(Kv8Value::Undefined);
            let mut all_args = m
                .get("__args")
                .map(array_values_of)
                .unwrap_or_default();
            all_args.extend(args);
            return call_value_with_this(ctx, target, all_args, Some(this_val));
        }
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
        "document.getElementById" => {
            let id = args.first().and_then(|v| v.as_str()).unwrap_or("");
            match ctx.get_element_by_id(id)? {
                Some(n) => Ok(Kv8Value::Dom(n)),
                None => Ok(Kv8Value::Null),
            }
        }
        "document.querySelectorAll" => {
            let sel = args.first().and_then(|v| v.as_str()).unwrap_or("div");
            node_list_object(ctx, ctx.query_selector_all(sel)?)
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
        "element.getRootNode" => ctx.owner_document_value(),
        "element.appendChild" => {
            let pid = m.get("__id").and_then(|v| v.as_num()).unwrap_or(0.0) as u64;
            if let Some(Kv8Value::Dom(child)) = args.first() {
                ctx.append_child(pid, child.clone())?;
            }
            Ok(Kv8Value::Null)
        }
        "element.removeChild" => {
            let pid = m.get("__id").and_then(|v| v.as_num()).unwrap_or(0.0) as u64;
            let child = args
                .first()
                .ok_or("removeChild(child) expects child node")?;
            let child_id = match child {
                Kv8Value::Dom(n) => n.id,
                _ => return Err("removeChild(child) expects DOM node".into()),
            };
            match ctx.remove_child(pid, child_id)? {
                Some(removed) => Ok(Kv8Value::Dom(removed)),
                None => Ok(Kv8Value::Null),
            }
        }
        "element.setAttribute" => {
            let node_id = m.get("__id").and_then(|v| v.as_num()).unwrap_or(0.0) as u64;
            let key = args
                .first()
                .and_then(|v| v.as_str())
                .ok_or("setAttribute(name, value) expects name")?;
            let value = args.get(1).map(kv8_value_to_string).unwrap_or_default();
            let mut node = ctx
                .resolve_node(node_id)?
                .ok_or_else(|| format!("setAttribute: node {node_id} not found"))?;
            node.set_attr(key, &value);
            ctx.publish_node(node)?;
            Ok(Kv8Value::Undefined)
        }
        "element.getAttribute" => {
            let node_id = m.get("__id").and_then(|v| v.as_num()).unwrap_or(0.0) as u64;
            let key = args
                .first()
                .and_then(|v| v.as_str())
                .ok_or("getAttribute(name) expects name")?;
            let value = ctx
                .resolve_node(node_id)?
                .and_then(|n| n.get_attr(key).map(str::to_string))
                .unwrap_or_default();
            Ok(Kv8Value::Str(value))
        }
        "element.querySelectorAll" => {
            let node_id = m.get("__id").and_then(|v| v.as_num()).unwrap_or(0.0) as u64;
            let sel = args.first().and_then(|v| v.as_str()).unwrap_or("div");
            node_list_object(ctx, ctx.query_selector_all_from(node_id, sel)?)
        }
        "NodeList.forEach" => {
            let list_id = m
                .get("__nodelist_id")
                .and_then(|v| v.as_num())
                .ok_or("NodeList.forEach missing list id")?
                as u64;
            let cb = args
                .first()
                .cloned()
                .ok_or("NodeList.forEach(callback) expects function")?;
            expect_kv8_fn(&cb, "NodeList.forEach")?;
            let nodes = ctx.nodelist_nodes(list_id)?;
            for (i, node) in nodes.into_iter().enumerate() {
                call_value(
                    ctx,
                    cb.clone(),
                    vec![Kv8Value::Dom(node), Kv8Value::Num(i as f64)],
                )?;
            }
            Ok(Kv8Value::Undefined)
        }
        "element.addEventListener" => {
            let node_id = m.get("__id").and_then(|v| v.as_num()).unwrap_or(0.0) as u64;
            let event_type = args
                .first()
                .and_then(|v| v.as_str())
                .ok_or("addEventListener(type, listener) expects type string")?;
            let listener = args
                .get(1)
                .cloned()
                .ok_or("addEventListener(type, listener) expects listener")?;
            if !is_kv8_callable(&listener) {
                return Err(format!(
                    "addEventListener listener must be a function ({})",
                    callee_debug_hint(&listener)
                ));
            }
            ctx.add_event_listener(node_id, event_type, listener)?;
            Ok(Kv8Value::Undefined)
        }
        "element.dispatchEvent" => {
            let node_id = m.get("__id").and_then(|v| v.as_num()).unwrap_or(0.0) as u64;
            let event = args
                .first()
                .cloned()
                .ok_or("dispatchEvent(event) expects event")?;
            let ok = dispatch_event_on_node(ctx, node_id, event)?;
            Ok(Kv8Value::Bool(ok))
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
        "setTimeout" => {
            let cb = args
                .first()
                .cloned()
                .ok_or("setTimeout(fn, delay) expects callback")?;
            expect_kv8_fn(&cb, "setTimeout")?;
            let delay = args.get(1).and_then(|v| v.as_num()).unwrap_or(0.0).max(0.0) as u64;
            let id = ctx.schedule_timer(cb, delay, None)?;
            Ok(Kv8Value::Num(id as f64))
        }
        "clearTimeout" => {
            let id = args
                .first()
                .and_then(|v| v.as_num())
                .ok_or("clearTimeout(id) expects number")?
                as u64;
            ctx.cancel_timer(id)?;
            Ok(Kv8Value::Undefined)
        }
        "setInterval" => {
            let cb = args
                .first()
                .cloned()
                .ok_or("setInterval(fn, delay) expects callback")?;
            expect_kv8_fn(&cb, "setInterval")?;
            let delay = args
                .get(1)
                .and_then(|v| v.as_num())
                .ok_or("setInterval(fn, delay) expects delay")?
                .max(0.0) as u64;
            let id = ctx.schedule_timer(cb, delay, Some(delay))?;
            Ok(Kv8Value::Num(id as f64))
        }
        "clearInterval" => {
            let id = args
                .first()
                .and_then(|v| v.as_num())
                .ok_or("clearInterval(id) expects number")?
                as u64;
            ctx.cancel_timer(id)?;
            Ok(Kv8Value::Undefined)
        }
        "requestAnimationFrame" => {
            let cb = args
                .first()
                .cloned()
                .ok_or("requestAnimationFrame(callback) expects callback")?;
            expect_kv8_fn(&cb, "requestAnimationFrame")?;
            let id = ctx.request_animation_frame(cb)?;
            Ok(Kv8Value::Num(id as f64))
        }
        "cancelAnimationFrame" => {
            let id = args
                .first()
                .and_then(|v| v.as_num())
                .ok_or("cancelAnimationFrame(id) expects number")?
                as u64;
            ctx.cancel_animation_frame(id)?;
            Ok(Kv8Value::Undefined)
        }
        "Promise.resolve" => {
            let v = args.first().cloned().unwrap_or(Kv8Value::Undefined);
            Ok(promise_resolved(v))
        }
        "Promise.reject" => {
            let msg = args
                .first()
                .map(kv8_value_to_string)
                .unwrap_or_else(|| "rejected".into());
            Ok(promise_rejected(msg))
        }
        "promise.resolve" => {
            let id = m
                .get("__promise_id")
                .and_then(|v| v.as_num())
                .ok_or("promise settler missing id")?
                as u64;
            let value = args.first().cloned().unwrap_or(Kv8Value::Undefined);
            if let Some(p) = promise_by_id(ctx, id) {
                kv8_settle_fulfilled(ctx, &p, value)?;
            }
            Ok(Kv8Value::Undefined)
        }
        "promise.reject" => {
            let id = m
                .get("__promise_id")
                .and_then(|v| v.as_num())
                .ok_or("promise settler missing id")?
                as u64;
            let msg = args
                .first()
                .map(kv8_value_to_string)
                .unwrap_or_else(|| "rejected".into());
            if let Some(p) = promise_by_id(ctx, id) {
                kv8_settle_rejected(ctx, &p, msg)?;
            }
            Ok(Kv8Value::Undefined)
        }
        "promise.then" => {
            let parent = m
                .get("__promise")
                .and_then(|v| match v {
                    Kv8Value::Promise(p) => Some(p.clone()),
                    _ => None,
                })
                .ok_or("promise.then missing promise")?;
            let on_fulfilled = args.first().cloned().filter(|v| !matches!(v, Kv8Value::Undefined));
            let on_rejected = args.get(1).cloned().filter(|v| !matches!(v, Kv8Value::Undefined));
            Ok(Kv8Value::Promise(attach_then(
                ctx,
                parent,
                on_fulfilled,
                on_rejected,
            )?))
        }
        "promise.catch" => {
            let parent = m
                .get("__promise")
                .and_then(|v| match v {
                    Kv8Value::Promise(p) => Some(p.clone()),
                    _ => None,
                })
                .ok_or("promise.catch missing promise")?;
            let on_rejected = args.first().cloned();
            Ok(Kv8Value::Promise(attach_then(ctx, parent, None, on_rejected)?))
        }
        "promise.runThenLink" => run_then_link_native(ctx, &m),
        "fetch" => {
            let url = args
                .first()
                .and_then(|v| v.as_str())
                .ok_or("fetch(url) expects url string")?;
            let promise = new_pending_promise();
            match kv8_http_fetch(url) {
                Ok(resp) => kv8_settle_fulfilled(ctx, &promise, resp)?,
                Err(e) => kv8_settle_rejected(ctx, &promise, e)?,
            }
            Ok(Kv8Value::Promise(promise))
        }
        "localStorage.setItem" => {
            let key = args
                .first()
                .and_then(|v| v.as_str())
                .ok_or("localStorage.setItem(key, value) expects key")?;
            let value = args
                .get(1)
                .and_then(|v| v.as_str())
                .ok_or("localStorage.setItem(key, value) expects value")?;
            ctx.storage_set(key, value)?;
            Ok(Kv8Value::Undefined)
        }
        "localStorage.getItem" => {
            let key = args
                .first()
                .and_then(|v| v.as_str())
                .ok_or("localStorage.getItem(key) expects key")?;
            match ctx.storage_get(key)? {
                Some(v) => Ok(Kv8Value::Str(v)),
                None => Ok(Kv8Value::Null),
            }
        }
        "localStorage.removeItem" => {
            let key = args
                .first()
                .and_then(|v| v.as_str())
                .ok_or("localStorage.removeItem(key) expects key")?;
            ctx.storage_remove(key)?;
            Ok(Kv8Value::Undefined)
        }
        "localStorage.clear" => {
            ctx.storage_clear()?;
            Ok(Kv8Value::Undefined)
        }
        "localStorage.key" => {
            let index = args
                .first()
                .and_then(|v| v.as_num())
                .ok_or("localStorage.key(index) expects number")?
                as usize;
            match ctx.storage_key(index)? {
                Some(k) => Ok(Kv8Value::Str(k)),
                None => Ok(Kv8Value::Null),
            }
        }
        "Array" => Ok(construct_array(args)),
        "Array.isArray" => {
            let v = args.first().cloned().unwrap_or(Kv8Value::Undefined);
            Ok(Kv8Value::Bool(is_kv8_array(&v)))
        }
        "Array.push" => {
            let added = args.len();
            ctx.with_mut(|inner| {
                let this = inner
                    .this_stack
                    .last_mut()
                    .ok_or("Array.push: no this")?;
                let Kv8Value::Obj(m) = this else {
                    return Err("Array.push: this is not an array".into());
                };
                let mut len = array_length_of(m);
                for arg in args {
                    m.insert(len.to_string(), arg);
                    len += 1;
                }
                m.insert("length".into(), Kv8Value::Num(len as f64));
                Ok(Kv8Value::Num(added as f64))
            })
        }
        "Array.pop" => ctx.with_mut(|inner| {
            let this = inner.this_stack.last_mut().ok_or("Array.pop: no this")?;
            let Kv8Value::Obj(m) = this else {
                return Err("Array.pop: this is not an array".into());
            };
            let len = array_length_of(m);
            if len == 0 {
                return Ok(Kv8Value::Undefined);
            }
            let key = (len - 1).to_string();
            let v = m.remove(&key).unwrap_or(Kv8Value::Undefined);
            m.insert("length".into(), Kv8Value::Num((len - 1) as f64));
            Ok(v)
        }),
        "Array.shift" => ctx.with_mut(|inner| {
            let this = inner.this_stack.last_mut().ok_or("Array.shift: no this")?;
            let Kv8Value::Obj(m) = this else {
                return Err("Array.shift: this is not an array".into());
            };
            let len = array_length_of(m);
            if len == 0 {
                return Ok(Kv8Value::Undefined);
            }
            let v = m.remove("0").unwrap_or(Kv8Value::Undefined);
            for i in 1..len {
                if let Some(item) = m.remove(&i.to_string()) {
                    m.insert((i - 1).to_string(), item);
                }
            }
            m.insert("length".into(), Kv8Value::Num((len - 1) as f64));
            Ok(v)
        }),
        "Array.unshift" => {
            let added = args.len();
            ctx.with_mut(|inner| {
                let this = inner.this_stack.last_mut().ok_or("Array.unshift: no this")?;
                let Kv8Value::Obj(m) = this else {
                    return Err("Array.unshift: this is not an array".into());
                };
                let len = array_length_of(m);
                for i in (0..len).rev() {
                    if let Some(item) = m.remove(&i.to_string()) {
                        m.insert((i + added).to_string(), item);
                    }
                }
                for (i, arg) in args.into_iter().enumerate() {
                    m.insert(i.to_string(), arg);
                }
                m.insert("length".into(), Kv8Value::Num((len + added) as f64));
                Ok(Kv8Value::Num((len + added) as f64))
            })
        }
        "Array.slice" => {
            let obj = current_this(ctx)?;
            let Kv8Value::Obj(m) = obj else {
                return Ok(array_from_values(vec![]));
            };
            let len = array_length_of(&m);
            let mut start = args
                .first()
                .and_then(|v| v.as_num())
                .unwrap_or(0.0) as i64;
            if start < 0 {
                start = len as i64 + start;
            }
            let start = start.clamp(0, len as i64) as usize;
            let end = args
                .get(1)
                .and_then(|v| v.as_num())
                .map(|n| {
                    let mut i = n.trunc() as i64;
                    if i < 0 {
                        i = len as i64 + i;
                    }
                    i.clamp(0, len as i64) as usize
                })
                .unwrap_or(len)
                .min(len);
            let mut out = Vec::new();
            for i in start..end {
                out.push(
                    m.get(&i.to_string())
                        .cloned()
                        .unwrap_or(Kv8Value::Undefined),
                );
            }
            Ok(array_from_values(out))
        }
        "Array.concat" => {
            let obj = current_this(ctx)?;
            let Kv8Value::Obj(m) = obj else {
                return Ok(array_from_values(vec![]));
            };
            let mut out = Vec::new();
            let len = array_length_of(&m);
            for i in 0..len {
                out.push(
                    m.get(&i.to_string())
                        .cloned()
                        .unwrap_or(Kv8Value::Undefined),
                );
            }
            for arg in args {
                if is_kv8_array(&arg) {
                    let Kv8Value::Obj(a) = arg else { continue };
                    let alen = array_length_of(&a);
                    for i in 0..alen {
                        out.push(
                            a.get(&i.to_string())
                                .cloned()
                                .unwrap_or(Kv8Value::Undefined),
                        );
                    }
                } else {
                    out.push(arg);
                }
            }
            Ok(array_from_values(out))
        }
        "Array.forEach" => {
            let callback = args
                .first()
                .cloned()
                .ok_or("Array.forEach expects callback")?;
            let this_arg = args.get(1).cloned().unwrap_or(Kv8Value::Undefined);
            let obj = current_this(ctx)?;
            let Kv8Value::Obj(m) = obj else {
                return Ok(Kv8Value::Undefined);
            };
            let len = array_length_of(&m);
            for i in 0..len {
                let v = m
                    .get(&i.to_string())
                    .cloned()
                    .unwrap_or(Kv8Value::Undefined);
                call_value_with_this(
                    ctx,
                    callback.clone(),
                    vec![v, Kv8Value::Num(i as f64), Kv8Value::Obj(m.clone())],
                    Some(this_arg.clone()),
                )?;
            }
            Ok(Kv8Value::Undefined)
        }
        "Array.map" => {
            let callback = args
                .first()
                .cloned()
                .ok_or("Array.map expects callback")?;
            let this_arg = args.get(1).cloned().unwrap_or(Kv8Value::Undefined);
            let obj = current_this(ctx)?;
            let Kv8Value::Obj(m) = obj else {
                return Ok(array_from_values(vec![]));
            };
            let len = array_length_of(&m);
            let mut out = Vec::new();
            for i in 0..len {
                let v = m
                    .get(&i.to_string())
                    .cloned()
                    .unwrap_or(Kv8Value::Undefined);
                out.push(call_value_with_this(
                    ctx,
                    callback.clone(),
                    vec![v, Kv8Value::Num(i as f64), Kv8Value::Obj(m.clone())],
                    Some(this_arg.clone()),
                )?);
            }
            Ok(array_from_values(out))
        }
        "Array.join" => {
            let obj = current_this(ctx)?;
            let Kv8Value::Obj(m) = obj else {
                return Ok(Kv8Value::Str(String::new()));
            };
            let sep = args.first().and_then(|v| v.as_str()).unwrap_or(",");
            let len = array_length_of(&m);
            let parts: Vec<String> = (0..len)
                .map(|i| {
                    m.get(&i.to_string())
                        .map(kv8_value_to_string)
                        .unwrap_or_default()
                })
                .collect();
            Ok(Kv8Value::Str(parts.join(sep)))
        }
        "Array.splice" => {
            ctx.with_mut(|inner| {
                let this = inner
                    .this_stack
                    .last_mut()
                    .ok_or("Array.splice: no this")?;
                let Kv8Value::Obj(m) = this else {
                    return Err("Array.splice: this is not an array".into());
                };
                let len = array_length_of(m);
                let start = args
                    .first()
                    .and_then(|v| v.as_num())
                    .unwrap_or(0.0) as i64;
                let start = start.clamp(0, len as i64) as usize;
                let delete_count = args
                    .get(1)
                    .and_then(|v| v.as_num())
                    .map(|n| n as i64)
                    .unwrap_or((len as i64 - start as i64).max(0));
                let delete_count = delete_count.clamp(0, (len - start) as i64) as usize;
                let mut removed = Vec::new();
                for i in start..start + delete_count {
                    removed.push(
                        m.remove(&i.to_string())
                            .unwrap_or(Kv8Value::Undefined),
                    );
                }
                let insert = args.iter().skip(2).cloned().collect::<Vec<_>>();
                let tail: Vec<Kv8Value> = (start + delete_count..len)
                    .map(|i| {
                        m.remove(&i.to_string())
                            .unwrap_or(Kv8Value::Undefined)
                    })
                    .collect();
                for (i, v) in insert.iter().enumerate() {
                    m.insert((start + i).to_string(), v.clone());
                }
                let tail_start = start + insert.len();
                for (i, v) in tail.into_iter().enumerate() {
                    m.insert((tail_start + i).to_string(), v);
                }
                let new_len = len - delete_count + insert.len();
                for i in new_len..len {
                    m.remove(&i.to_string());
                }
                m.insert("length".into(), Kv8Value::Num(new_len as f64));
                Ok(array_from_values(removed))
            })
        }
        "Map" => construct_map_from_args(args),
        "WeakMap" => Ok(weak_map_from_entries(vec![])),
        "Set" => construct_set_from_args(args),
        "FormData" => Ok(Kv8Value::Obj({
            let mut m = HashMap::new();
            m.insert("__native".into(), Kv8Value::Str("FormData".into()));
            m
        })),
        "performance.now" => Ok(Kv8Value::Num(
            crate::value::unix_ms_now() as f64,
        )),
        "performance.getEntriesByType" => Ok(array_from_values(vec![])),
        "Date.now" => Ok(Kv8Value::Num(crate::value::unix_ms_now() as f64)),
        "Math.random" => {
            use rand::Rng;
            Ok(Kv8Value::Num(rand::thread_rng().gen::<f64>()))
        }
        "Math.floor" => {
            let n = args.first().and_then(|v| v.as_num()).unwrap_or(0.0);
            Ok(Kv8Value::Num(n.floor()))
        }
        "Math.min" => {
            let mut min = f64::INFINITY;
            for a in &args {
                if let Some(n) = a.as_num() {
                    min = min.min(n);
                }
            }
            Ok(Kv8Value::Num(if min.is_finite() { min } else { 0.0 }))
        }
        "Math.log" => {
            let n = args.first().and_then(|v| v.as_num()).unwrap_or(1.0);
            Ok(Kv8Value::Num(n.ln()))
        }
        "Math.clz32" => {
            let n = args
                .first()
                .and_then(|v| v.as_num())
                .unwrap_or(0.0) as u32;
            Ok(Kv8Value::Num(n.leading_zeros() as f64))
        }
        "encodeURIComponent" => {
            let s = args
                .first()
                .map(kv8_value_to_string)
                .unwrap_or_default();
            Ok(Kv8Value::Str(js_encode_uri_component(&s)))
        }
        "isNaN" => {
            let n = args.first().and_then(|v| v.as_num());
            Ok(Kv8Value::Bool(match n {
                Some(x) => x.is_nan(),
                None => true,
            }))
        }
        "RegExp" => {
            let pattern = args
                .first()
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let mut m = HashMap::new();
            m.insert("__native".into(), Kv8Value::Str("RegExp".into()));
            m.insert("__pattern".into(), Kv8Value::Str(pattern));
            Ok(Kv8Value::Obj(m))
        }
        "RegExp.test" => {
            let obj = current_this(ctx)?;
            let Kv8Value::Obj(m) = obj else {
                return Ok(Kv8Value::Bool(false));
            };
            let pattern = m
                .get("__pattern")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let input = args
                .first()
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let ok = regex::Regex::new(pattern)
                .map(|re| re.is_match(input))
                .unwrap_or(false);
            Ok(Kv8Value::Bool(ok))
        }
        "Number.toString" => {
            let n = current_this(ctx)?;
            let Kv8Value::Num(n) = n else {
                return Ok(Kv8Value::Str(String::new()));
            };
            let radix = args
                .first()
                .and_then(|v| v.as_num())
                .unwrap_or(10.0) as u32;
            if radix == 10 {
                let s = if n.fract() == 0.0 {
                    format!("{}", n as i64)
                } else {
                    n.to_string()
                };
                Ok(Kv8Value::Str(s))
            } else {
                Ok(Kv8Value::Str(num_to_radix(n, radix)))
            }
        }
        "String.slice" => {
            let s = match current_this(ctx)? {
                Kv8Value::Str(s) => s,
                _ => return Ok(Kv8Value::Str(String::new())),
            };
            let start = args.first().and_then(|v| v.as_num()).unwrap_or(0.0);
            let end = args.get(1).and_then(|v| v.as_num());
            let chars: Vec<char> = s.chars().collect();
            let len = chars.len();
            let mut a = js_str_index(&s, start);
            let b = end
                .map(|e| js_str_index(&s, e))
                .unwrap_or(len)
                .min(len);
            if a > b {
                a = b;
            }
            Ok(Kv8Value::Str(chars[a..b].iter().collect()))
        }
        "String.indexOf" => {
            let s = match current_this(ctx)? {
                Kv8Value::Str(s) => s,
                _ => return Ok(Kv8Value::Num(-1.0)),
            };
            let needle = args.first().and_then(|v| v.as_str()).unwrap_or("");
            let from = args
                .get(1)
                .and_then(|v| v.as_num())
                .map(|n| js_str_index(&s, n))
                .unwrap_or(0);
            let from_byte = s.char_indices().nth(from).map(|(i, _)| i).unwrap_or(s.len());
            match s[from_byte..].find(needle) {
                Some(i) => Ok(Kv8Value::Num((from_byte + i) as f64)),
                None => Ok(Kv8Value::Num(-1.0)),
            }
        }
        "String.includes" => {
            let s = match current_this(ctx)? {
                Kv8Value::Str(s) => s,
                _ => return Ok(Kv8Value::Bool(false)),
            };
            let needle = args.first().and_then(|v| v.as_str()).unwrap_or("");
            Ok(Kv8Value::Bool(s.contains(needle)))
        }
        "String.split" => {
            let s = match current_this(ctx)? {
                Kv8Value::Str(s) => s,
                _ => return Ok(array_from_values(vec![])),
            };
            let sep = args.first().and_then(|v| v.as_str()).unwrap_or("");
            let parts: Vec<Kv8Value> = if sep.is_empty() {
                s.chars()
                    .map(|c| Kv8Value::Str(c.to_string()))
                    .collect()
            } else {
                s.split(sep).map(|p| Kv8Value::Str(p.to_string())).collect()
            };
            Ok(array_from_values(parts))
        }
        "String.replace" => {
            let s = match current_this(ctx)? {
                Kv8Value::Str(s) => s,
                _ => return Ok(Kv8Value::Str(String::new())),
            };
            let from = args.first().and_then(|v| v.as_str()).unwrap_or("");
            let to = args.get(1).and_then(|v| v.as_str()).unwrap_or("");
            Ok(Kv8Value::Str(s.replacen(from, to, 1)))
        }
        "String.trim" => {
            let s = match current_this(ctx)? {
                Kv8Value::Str(s) => s,
                _ => return Ok(Kv8Value::Str(String::new())),
            };
            Ok(Kv8Value::Str(s.trim().to_string()))
        }
        "String.toLowerCase" => {
            let s = match current_this(ctx)? {
                Kv8Value::Str(s) => s,
                _ => return Ok(Kv8Value::Str(String::new())),
            };
            Ok(Kv8Value::Str(s.to_lowercase()))
        }
        "String.toUpperCase" => {
            let s = match current_this(ctx)? {
                Kv8Value::Str(s) => s,
                _ => return Ok(Kv8Value::Str(String::new())),
            };
            Ok(Kv8Value::Str(s.to_uppercase()))
        }
        "String.charCodeAt" => {
            let s = match current_this(ctx)? {
                Kv8Value::Str(s) => s,
                _ => return Ok(Kv8Value::Num(f64::NAN)),
            };
            let idx = js_str_index(&s, args.first().and_then(|v| v.as_num()).unwrap_or(0.0));
            let code = s.chars().nth(idx).map(|c| c as u32).unwrap_or(0);
            Ok(Kv8Value::Num(code as f64))
        }
        "String.match" => {
            let s = match current_this(ctx)? {
                Kv8Value::Str(s) => s,
                _ => return Ok(Kv8Value::Null),
            };
            let re = args.first().ok_or("String.match expects RegExp")?;
            let pattern = re
                .as_obj()
                .and_then(|m| m.get("__pattern"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let re = regex::Regex::new(pattern).ok();
            let Some(re) = re else {
                return Ok(Kv8Value::Null);
            };
            if let Some(m) = re.find(&s) {
                Ok(array_from_values(vec![Kv8Value::Str(m.as_str().to_string())]))
            } else {
                Ok(Kv8Value::Null)
            }
        }
        "String.fromCharCode" => {
            let mut out = String::new();
            for a in &args {
                if let Some(n) = a.as_num() {
                    if let Some(ch) = char::from_u32(n as u32) {
                        out.push(ch);
                    }
                }
            }
            Ok(Kv8Value::Str(out))
        }
        "Map.get" => {
            let key = index_to_key(
                args.first()
                    .ok_or("Map.get(key) expects key")?,
            );
            let obj = current_this(ctx)?;
            let Kv8Value::Obj(m) = obj else {
                return Ok(Kv8Value::Undefined);
            };
            Ok(m.get(&key)
                .cloned()
                .unwrap_or(Kv8Value::Undefined))
        }
        "Map.set" => {
            let key = index_to_key(
                args.first()
                    .ok_or("Map.set(key,value) expects key")?,
            );
            let val = args.get(1).cloned().unwrap_or(Kv8Value::Undefined);
            ctx.with_mut(|inner| {
                let this = inner
                    .this_stack
                    .last_mut()
                    .ok_or("Map.set: no this")?;
                let Kv8Value::Obj(m) = this else {
                    return Err("Map.set: this is not a Map".into());
                };
                m.insert(key, val);
                Ok(Kv8Value::Undefined)
            })
        }
        "Map.has" => {
            let key = index_to_key(
                args.first()
                    .ok_or("Map.has(key) expects key")?,
            );
            let obj = current_this(ctx)?;
            let Kv8Value::Obj(m) = obj else {
                return Ok(Kv8Value::Bool(false));
            };
            Ok(Kv8Value::Bool(m.contains_key(&key)))
        }
        "Map.delete" => {
            let key = index_to_key(
                args.first()
                    .ok_or("Map.delete(key) expects key")?,
            );
            ctx.with_mut(|inner| {
                let this = inner
                    .this_stack
                    .last_mut()
                    .ok_or("Map.delete: no this")?;
                let Kv8Value::Obj(m) = this else {
                    return Ok(Kv8Value::Bool(false));
                };
                Ok(Kv8Value::Bool(m.remove(&key).is_some()))
            })
        }
        "Map.forEach" => {
            let callback = args
                .first()
                .cloned()
                .ok_or("Map.forEach expects callback")?;
            let this_arg = args.get(1).cloned().unwrap_or(Kv8Value::Undefined);
            let obj = current_this(ctx)?;
            let Kv8Value::Obj(m) = obj else {
                return Ok(Kv8Value::Undefined);
            };
            let entries: Vec<(String, Kv8Value)> = m
                .iter()
                .filter(|(k, _)| is_map_entry_key(k))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            let map_val = Kv8Value::Obj(m);
            for (k, v) in entries {
                call_value_with_this(
                    ctx,
                    callback.clone(),
                    vec![v, Kv8Value::Str(k), map_val.clone()],
                    Some(this_arg.clone()),
                )?;
            }
            Ok(Kv8Value::Undefined)
        }
        "Set.add" => {
            let val = args
                .first()
                .cloned()
                .unwrap_or(Kv8Value::Undefined);
            ctx.with_mut(|inner| {
                let this = inner
                    .this_stack
                    .last_mut()
                    .ok_or("Set.add: no this")?;
                let Kv8Value::Obj(m) = this else {
                    return Err("Set.add: this is not a Set".into());
                };
                m.insert(set_storage_key(&val), val);
                Ok(Kv8Value::Undefined)
            })
        }
        "Set.has" => {
            let val = args
                .first()
                .cloned()
                .unwrap_or(Kv8Value::Undefined);
            let obj = current_this(ctx)?;
            let Kv8Value::Obj(m) = obj else {
                return Ok(Kv8Value::Bool(false));
            };
            Ok(Kv8Value::Bool(
                m.contains_key(&set_storage_key(&val)),
            ))
        }
        "Set.delete" => {
            let val = args
                .first()
                .cloned()
                .unwrap_or(Kv8Value::Undefined);
            ctx.with_mut(|inner| {
                let this = inner
                    .this_stack
                    .last_mut()
                    .ok_or("Set.delete: no this")?;
                let Kv8Value::Obj(m) = this else {
                    return Ok(Kv8Value::Bool(false));
                };
                Ok(Kv8Value::Bool(
                    m.remove(&set_storage_key(&val)).is_some(),
                ))
            })
        }
        "Set.forEach" => {
            let callback = args
                .first()
                .cloned()
                .ok_or("Set.forEach expects callback")?;
            let this_arg = args.get(1).cloned().unwrap_or(Kv8Value::Undefined);
            let obj = current_this(ctx)?;
            let Kv8Value::Obj(m) = obj else {
                return Ok(Kv8Value::Undefined);
            };
            let values: Vec<Kv8Value> = m
                .iter()
                .filter(|(k, _)| k.starts_with('\x01'))
                .map(|(_, v)| v.clone())
                .collect();
            let set_val = Kv8Value::Obj(m);
            for v in values {
                let v2 = v.clone();
                call_value_with_this(
                    ctx,
                    callback.clone(),
                    vec![v, v2, set_val.clone()],
                    Some(this_arg.clone()),
                )?;
            }
            Ok(Kv8Value::Undefined)
        }
        "Object.assign" => {
            let mut target = args
                .first()
                .cloned()
                .unwrap_or_else(|| Kv8Value::Obj(HashMap::new()));
            if let Kv8Value::Obj(ref mut t) = target {
                for src in args.iter().skip(1) {
                    if let Kv8Value::Obj(src_m) = src {
                        for (k, v) in src_m {
                            if !k.starts_with("__") {
                                t.insert(k.clone(), v.clone());
                            }
                        }
                    }
                }
            }
            Ok(target)
        }
        "Object.create" => {
            let proto = args
                .first()
                .cloned()
                .unwrap_or(Kv8Value::Undefined);
            object_create(ctx, proto)
        }
        "Object.defineProperty" => {
            let target = args
                .first()
                .cloned()
                .unwrap_or(Kv8Value::Undefined);
            let key = args
                .get(1)
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let desc = args
                .get(2)
                .cloned()
                .unwrap_or_else(|| Kv8Value::Obj(HashMap::new()));
            object_define_property(ctx, target, key, desc)
        }
        "Object.getOwnPropertyDescriptor" => {
            let obj = args
                .first()
                .cloned()
                .unwrap_or(Kv8Value::Undefined);
            let key = args
                .get(1)
                .and_then(|v| v.as_str())
                .unwrap_or("");
            object_get_own_property_descriptor(ctx, obj, key)
        }
        "Object.getOwnPropertyNames" => {
            let obj = args
                .first()
                .cloned()
                .unwrap_or(Kv8Value::Undefined);
            object_own_property_names(ctx, obj)
        }
        "Object.getPrototypeOf" => {
            let obj = args
                .first()
                .cloned()
                .unwrap_or(Kv8Value::Undefined);
            object_get_prototype_of(obj)
        }
        "Object.prototype.hasOwnProperty" => {
            let obj = current_this(ctx)?;
            let key = args.first().and_then(|v| v.as_str()).unwrap_or("");
            let has = match obj {
                Kv8Value::Obj(m) => {
                    if m.contains_key(key) {
                        true
                    } else if let Some(id) = plain_obj_id(&m) {
                        ctx.with_mut(|inner| {
                            Ok(inner
                                .obj_store
                                .get(&id)
                                .map(|store| {
                                    store.contains_key(key)
                                        || store.contains_key(&object_desc_store_key(key))
                                })
                                .unwrap_or(false))
                        })?
                    } else {
                        false
                    }
                }
                _ => false,
            };
            Ok(Kv8Value::Bool(has))
        }
        "Symbol.for" => {
            let key = args.first().and_then(|v| v.as_str()).unwrap_or("");
            ctx.symbol_for(key)
        }
        "Error" => {
            let msg = args
                .first()
                .map(kv8_value_to_string)
                .unwrap_or_default();
            let mut m = HashMap::new();
            m.insert("__native".into(), Kv8Value::Str("Error.instance".into()));
            m.insert("message".into(), Kv8Value::Str(msg));
            Ok(Kv8Value::Obj(m))
        }
        _ => Err(format!("unknown native call: {native}")),
    }
}

fn event_type_from(event: &Kv8Value) -> Option<String> {
    match event {
        Kv8Value::Str(s) => Some(s.clone()),
        Kv8Value::Obj(map) => map.get("type").and_then(|v| v.as_str()).map(str::to_string),
        _ => None,
    }
}

fn build_event_object(event_type: &str, target: DomNode) -> Kv8Value {
    let mut map = HashMap::new();
    map.insert("type".into(), Kv8Value::Str(event_type.to_string()));
    map.insert("target".into(), Kv8Value::Dom(target));
    Kv8Value::Obj(map)
}

fn dispatch_event_on_node(
    ctx: &Kv8Context,
    node_id: u64,
    event: Kv8Value,
) -> Result<bool, String> {
    let event_type = event_type_from(&event).ok_or("dispatchEvent: missing event type")?;
    let event_obj = match &event {
        Kv8Value::Obj(map) if map.contains_key("type") => event.clone(),
        _ => {
            let target = ctx
                .find_dom_by_id(node_id)?
                .unwrap_or_else(|| DomNode::element("unknown"));
            build_event_object(&event_type, target)
        }
    };
    let listeners = ctx.listeners_for(node_id, &event_type)?;
    for listener in listeners {
        invoke_listener(ctx, listener, event_obj.clone())?;
    }
    Ok(true)
}

fn invoke_listener(ctx: &Kv8Context, listener: Kv8Value, event: Kv8Value) -> Result<(), String> {
    let params_empty = match &listener {
        Kv8Value::Arrow { params, .. }
        | Kv8Value::Fun { params, .. }
        | Kv8Value::AsyncFun { params, .. } => params.is_empty(),
        _ => true,
    };
    let args = if params_empty {
        vec![]
    } else {
        vec![event]
    };
    let _ = call_value(ctx, listener, args)?;
    Ok(())
}

fn is_kv8_callable(v: &Kv8Value) -> bool {
    match v {
        Kv8Value::Fun { .. } | Kv8Value::Arrow { .. } | Kv8Value::AsyncFun { .. } => true,
        Kv8Value::Obj(m) => m
            .get("__native")
            .and_then(|n| n.as_str())
            .is_some_and(|n| {
                n == "bound.call"
                    || n.starts_with("element.")
                    || n.starts_with("document.")
                    || n.starts_with("Array.")
                    || n.starts_with("Map.")
                    || n.starts_with("Promise.")
                    || n.starts_with("console.")
            }),
        _ => false,
    }
}

fn expect_kv8_fn(v: &Kv8Value, api: &str) -> Result<(), String> {
    if is_kv8_callable(v) {
        Ok(())
    } else {
        Err(format!("{api} expects function"))
    }
}

fn construct_array(args: Vec<Kv8Value>) -> Kv8Value {
    if args.len() == 1 {
        if let Some(n) = args[0].as_num() {
            if n.fract() == 0.0 && n >= 0.0 {
                return array_with_length(n as usize);
            }
        }
    }
    array_from_values(args)
}

fn construct_new(ctx: &Kv8Context, callee: Kv8Value, args: Vec<Kv8Value>) -> Result<Kv8Value, String> {
    if let Kv8Value::Fun {
        params,
        body,
        prototype,
        ..
    } = callee
    {
        return construct_function_new(ctx, params, body, prototype, args);
    }
    if let Kv8Value::AsyncFun {
        params,
        body,
        prototype,
        ..
    } = callee
    {
        return construct_function_new(ctx, params, body, prototype, args);
    }
    let native = callee
        .as_obj()
        .and_then(|m| m.get("__native"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    match native {
        "Promise" => construct_promise(ctx, args),
        "Array" => Ok(construct_array(args)),
        "Map" => construct_map_from_args(args),
        "WeakMap" => Ok(weak_map_from_entries(vec![])),
        "Set" => construct_set_from_args(args),
        "FormData" => Ok(Kv8Value::Obj({
            let mut m = HashMap::new();
            m.insert("__native".into(), Kv8Value::Str("FormData".into()));
            m
        })),
        "RegExp" => {
            let pattern = args
                .first()
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let mut m = HashMap::new();
            m.insert("__native".into(), Kv8Value::Str("RegExp".into()));
            m.insert("__pattern".into(), Kv8Value::Str(pattern));
            Ok(Kv8Value::Obj(m))
        }
        _ => Err(format!(
            "unknown constructor: {}",
            callee_debug_hint(&callee)
        )),
    }
}

fn construct_function_new(
    ctx: &Kv8Context,
    params: Vec<Kv8Param>,
    body: Vec<Stmt>,
    prototype: HashMap<String, Kv8Value>,
    args: Vec<Kv8Value>,
) -> Result<Kv8Value, String> {
    let mut instance_map = HashMap::new();
    for (k, v) in &prototype {
        if !k.starts_with("__") {
            instance_map.insert(k.clone(), v.clone());
        }
    }
    if !prototype.is_empty() {
        instance_map.insert("__proto__".into(), Kv8Value::Obj(prototype));
    }
    register_plain_obj(ctx, &mut instance_map);
    let instance = Kv8Value::Obj(instance_map);
    ctx.with_mut(|inner| {
        inner.scope_push();
        Ok(())
    })?;
    push_this(ctx, instance.clone())?;
    bind_function_params(ctx, &params, &args)?;
    let result = flow_fn_result(run_stmts(ctx, &body)?);
    pop_this(ctx)?;
    ctx.with_mut(|inner| {
        inner.scope_pop();
        Ok(())
    })?;
    match result {
        Ok(Kv8Value::Obj(_)) => result,
        Ok(_) => Ok(instance),
        Err(e) => Err(e),
    }
}

fn construct_promise(ctx: &Kv8Context, args: Vec<Kv8Value>) -> Result<Kv8Value, String> {
    let executor = args
        .first()
        .cloned()
        .ok_or("Promise constructor expects executor")?;
    expect_kv8_fn(&executor, "Promise")?;
    let promise = new_pending_promise();
    let resolve = make_promise_settler(ctx, promise.clone(), true);
    let reject = make_promise_settler(ctx, promise.clone(), false);
    call_value(ctx, executor, vec![resolve, reject])?;
    Ok(Kv8Value::Promise(promise))
}

fn make_promise_settler(
    ctx: &Kv8Context,
    promise: super::promise::SharedKv8Promise,
    resolve: bool,
) -> Kv8Value {
    let id = ctx
        .with_mut(|inner| {
            let id = inner.next_promise_handle;
            inner.next_promise_handle = inner.next_promise_handle.saturating_add(1);
            inner.promise_handles.insert(id, promise);
            Ok(id)
        })
        .unwrap_or(0);
    let mut m = HashMap::new();
    m.insert(
        "__native".into(),
        Kv8Value::Str(if resolve {
            "promise.resolve".into()
        } else {
            "promise.reject".into()
        }),
    );
    m.insert("__promise_id".into(), Kv8Value::Num(id as f64));
    Kv8Value::Obj(m)
}

fn promise_by_id(ctx: &Kv8Context, id: u64) -> Option<super::promise::SharedKv8Promise> {
    ctx.with_mut(|inner| Ok(inner.promise_handles.get(&id).cloned()))
        .ok()
        .flatten()
}

fn kv8_settle_fulfilled(
    ctx: &Kv8Context,
    promise: &super::promise::SharedKv8Promise,
    value: Kv8Value,
) -> Result<(), String> {
    let links = fulfill_promise(promise, value.clone()).unwrap_or_default();
    for link in links {
        enqueue_then_link(ctx, link, Ok(value.clone()))?;
    }
    Ok(())
}

fn kv8_settle_rejected(
    ctx: &Kv8Context,
    promise: &super::promise::SharedKv8Promise,
    message: impl Into<String>,
) -> Result<(), String> {
    let msg = message.into();
    let links = reject_promise(promise, msg.clone()).unwrap_or_default();
    for link in links {
        enqueue_then_link(ctx, link, Err(msg.clone()))?;
    }
    Ok(())
}

fn attach_then(
    ctx: &Kv8Context,
    parent: super::promise::SharedKv8Promise,
    on_fulfilled: Option<Kv8Value>,
    on_rejected: Option<Kv8Value>,
) -> Result<super::promise::SharedKv8Promise, String> {
    let child = new_pending_promise();
    let link = Kv8ThenLink {
        child: child.clone(),
        on_fulfilled,
        on_rejected,
    };
    match promise_state(&parent) {
        Kv8PromiseState::Pending => push_then_link(&parent, link),
        Kv8PromiseState::Fulfilled(v) => enqueue_then_link(ctx, link, Ok(v))?,
        Kv8PromiseState::Rejected(e) => enqueue_then_link(ctx, link, Err(e))?,
    }
    Ok(child)
}

fn enqueue_then_link(
    ctx: &Kv8Context,
    link: Kv8ThenLink,
    result: Result<Kv8Value, String>,
) -> Result<(), String> {
    let mut task = HashMap::new();
    task.insert(
        "__native".into(),
        Kv8Value::Str("promise.runThenLink".into()),
    );
    task.insert("__child".into(), Kv8Value::Promise(link.child));
    if let Some(f) = link.on_fulfilled {
        task.insert("__onFulfilled".into(), f);
    }
    if let Some(f) = link.on_rejected {
        task.insert("__onRejected".into(), f);
    }
    match result {
        Ok(v) => {
            task.insert("__reject".into(), Kv8Value::Bool(false));
            task.insert("__value".into(), v);
        }
        Err(e) => {
            task.insert("__reject".into(), Kv8Value::Bool(true));
            task.insert("__error".into(), Kv8Value::Str(e));
        }
    }
    ctx.enqueue_microtask(Kv8Value::Obj(task), vec![])
}

fn run_then_link_native(
    ctx: &Kv8Context,
    m: &HashMap<String, Kv8Value>,
) -> Result<Kv8Value, String> {
    let child = m
        .get("__child")
        .and_then(|v| match v {
            Kv8Value::Promise(p) => Some(p.clone()),
            _ => None,
        })
        .ok_or("promise.runThenLink missing child")?;
    let rejected = m
        .get("__reject")
        .and_then(|v| match v {
            Kv8Value::Bool(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(false);
    if rejected {
        let err = m
            .get("__error")
            .map(kv8_value_to_string)
            .unwrap_or_else(|| "rejected".into());
        if let Some(f) = m.get("__onRejected") {
            match call_value(ctx, f.clone(), vec![Kv8Value::Str(err.clone())]) {
                Ok(v) => kv8_settle_fulfilled(ctx, &child, v)?,
                Err(e) => kv8_settle_rejected(ctx, &child, e)?,
            }
        } else {
            kv8_settle_rejected(ctx, &child, err)?;
        }
    } else {
        let value = m
            .get("__value")
            .cloned()
            .unwrap_or(Kv8Value::Undefined);
        if let Some(f) = m.get("__onFulfilled") {
            match call_value(ctx, f.clone(), vec![value.clone()]) {
                Ok(v) => kv8_settle_fulfilled(ctx, &child, v)?,
                Err(e) => kv8_settle_rejected(ctx, &child, e)?,
            }
        } else {
            kv8_settle_fulfilled(ctx, &child, value)?;
        }
    }
    Ok(Kv8Value::Undefined)
}

fn await_value(ctx: &Kv8Context, value: Kv8Value) -> Result<Kv8Value, String> {
    match value {
        Kv8Value::Promise(p) => match promise_state(&p) {
            Kv8PromiseState::Fulfilled(v) => Ok(v),
            Kv8PromiseState::Rejected(e) => Err(e),
            Kv8PromiseState::Pending => {
                drain_event_loop(ctx)?;
                match promise_state(&p) {
                    Kv8PromiseState::Fulfilled(v) => Ok(v),
                    Kv8PromiseState::Rejected(e) => Err(e),
                    Kv8PromiseState::Pending => Err("await on pending promise".into()),
                }
            }
        },
        other => Ok(other),
    }
}

fn kv8_value_to_string(v: &Kv8Value) -> String {
    match v {
        Kv8Value::Str(s) => s.clone(),
        Kv8Value::Num(n) => n.to_string(),
        Kv8Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

pub fn drain_microtasks(ctx: &Kv8Context) -> Result<i64, String> {
    let mut count = 0i64;
    const MAX_ROUNDS: usize = 1024;
    for _ in 0..MAX_ROUNDS {
        let batch = ctx.with_mut(|inner| Ok(inner.microtasks.drain(..).collect::<Vec<_>>()))?;
        if batch.is_empty() {
            break;
        }
        for task in batch {
            if task.args.is_empty() {
                let _ = call_value(ctx, task.callback, vec![])?;
            } else {
                let _ = call_value(ctx, task.callback, task.args)?;
            }
            count += 1;
        }
    }
    Ok(count)
}

/// Drain microtasks then animation frames then timers until idle.
pub fn drain_event_loop(ctx: &Kv8Context) -> Result<i64, String> {
    let mut total = 0i64;
    for _ in 0..64 {
        let m = drain_microtasks(ctx)?;
        let r = drain_animation_frames(ctx)?;
        let t = drain_timers(ctx)?;
        total += m + r + t;
        if m == 0 && r == 0 && t == 0 {
            break;
        }
    }
    Ok(total)
}

/// Run all queued `requestAnimationFrame` callbacks (one frame).
pub fn drain_animation_frames(ctx: &Kv8Context) -> Result<i64, String> {
    let batch = ctx.take_raf_callbacks()?;
    if batch.is_empty() {
        return Ok(0);
    }
    let ts = crate::value::unix_ms_now() as f64;
    let mut count = 0i64;
    for cb in batch {
        invoke_listener(ctx, cb, Kv8Value::Num(ts))?;
        count += 1;
    }
    Ok(count)
}

/// Run all timer callbacks whose deadline has passed (call after `setTimeout` / `setInterval`).
pub fn drain_timers(ctx: &Kv8Context) -> Result<i64, String> {
    let mut count = 0i64;
    const MAX_ROUNDS: usize = 1024;
    for _ in 0..MAX_ROUNDS {
        let batch = ctx.take_due_timers()?;
        if batch.is_empty() {
            break;
        }
        for cb in batch {
            invoke_listener(ctx, cb, Kv8Value::Undefined)?;
            count += 1;
        }
    }
    Ok(count)
}

pub fn dom_to_kabootar(node: &DomNode) -> crate::value::Value {
    crate::value::Value::KabootarDom(node.clone())
}
