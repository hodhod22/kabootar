//! Compile Kv8 arrow/function bodies to Kabootar bytecode.

use super::ast::{Expr, Kv8Program, Stmt};
use crate::bytecode::{compile_source, run_bytecode_fn, BytecodeModule};
use crate::value::{BytecodeFunction, Environment, Value};
use std::rc::Rc;

#[derive(Clone)]
pub struct Kv8BytecodeFn {
    pub func: BytecodeFunction,
}

pub fn compile_program(program: &Kv8Program) -> Result<Option<BytecodeModule>, String> {
    let kabootar = program_to_kabootar(program);
    let compiled = compile_source(&kabootar)?;
    Ok(compiled.bytecode)
}

pub fn compile_arrow(params: &[String], body: &Expr) -> Result<Kv8BytecodeFn, String> {
    let kabootar = format!(
        "fn kv8_arrow_fn({}) {{ return {} }}",
        params.join(", "),
        expr_to_kabootar(body)
    );
    let compiled = compile_source(&kabootar)?;
    let bc = compiled
        .bytecode
        .ok_or("kv8 arrow did not compile to bytecode")?;
    let func_def = bc
        .functions
        .iter()
        .find(|f| f.name == "kv8_arrow_fn")
        .ok_or("kv8 arrow function missing")?;
    Ok(Kv8BytecodeFn {
        func: BytecodeFunction {
            def: Rc::new(func_def.clone()),
            closure: Environment::new(),
        },
    })
}

pub fn run_kv8_bytecode_fn(func: &Kv8BytecodeFn, args: Vec<Value>, env: &mut Environment) -> Result<Value, String> {
    run_bytecode_fn(func.func.def.as_ref(), args, env)
}

fn program_to_kabootar(program: &Kv8Program) -> String {
    let mut out = String::new();
    for stmt in &program.stmts {
        out.push_str(&stmt_to_kabootar(stmt));
        out.push('\n');
    }
    out
}

fn stmt_to_kabootar(stmt: &Stmt) -> String {
    match stmt {
        Stmt::Let(name, expr) => format!("let {name} = {};", expr_to_kabootar(expr)),
        Stmt::Return(expr) => format!("return {};", expr_to_kabootar(expr)),
        Stmt::Expr(expr) => format!("{};", expr_to_kabootar(expr)),
        Stmt::If(cond, then_b, else_b) => {
            let mut s = format!("if ({}) {{ ", expr_to_kabootar(cond));
            for st in then_b {
                s.push_str(&stmt_to_kabootar(st));
            }
            s.push_str(" }");
            if let Some(e) = else_b {
                s.push_str(" else { ");
                for st in e {
                    s.push_str(&stmt_to_kabootar(st));
                }
                s.push_str(" }");
            }
            s
        }
        Stmt::For(var, start, cond, step, body) => {
            let mut s = format!(
                "for (let {var} = {}; {}; {var} = {}) {{ ",
                expr_to_kabootar(start),
                expr_to_kabootar(cond),
                expr_to_kabootar(step),
            );
            for st in body {
                s.push_str(&stmt_to_kabootar(st));
            }
            s.push_str(" }");
            s
        }
        Stmt::Function(name, params, body) => {
            let mut s = format!("fn {name}({}) {{ ", params.join(", "));
            for st in body {
                s.push_str(&stmt_to_kabootar(st));
            }
            s.push('}');
            s
        }
        Stmt::Assign(lv, expr) => format!("{} = {};", lvalue_to_kabootar(lv), expr_to_kabootar(expr)),
    }
}

fn expr_to_kabootar(expr: &Expr) -> String {
    match expr {
        Expr::Lit(v) => super::ast::literal_to_string(v),
        Expr::Var(n) => n.clone(),
        Expr::Member(base, f) => format!("{}.{}", expr_to_kabootar(base), f),
        Expr::Call(c, args) => {
            let a = args.iter().map(expr_to_kabootar).collect::<Vec<_>>().join(", ");
            format!("{}({a})", expr_to_kabootar(c))
        }
        Expr::Bin(l, op, r) => format!(
            "({} {} {})",
            expr_to_kabootar(l),
            op,
            expr_to_kabootar(r)
        ),
        Expr::Unary(op, inner) => format!("({op}{})", expr_to_kabootar(inner)),
        Expr::Arrow(params, body) => {
            format!(
                "fn kv8_arrow_fn({}) {{ return {} }}",
                params.join(", "),
                expr_to_kabootar(body)
            )
        }
        Expr::Block(stmts) => {
            let mut s = String::from("{ ");
            for st in stmts {
                s.push_str(&stmt_to_kabootar(st));
            }
            s.push_str(" }");
            s
        }
    }
}

fn lvalue_to_kabootar(lv: &super::ast::LValue) -> String {
    match lv {
        super::ast::LValue::Name(n) => n.clone(),
        super::ast::LValue::Member(base, f) => format!("{}.{}", lvalue_to_kabootar(base), f),
    }
}

pub fn try_compile_loop_body(body: &[Stmt]) -> Result<Option<Kv8BytecodeFn>, String> {
    let kabootar: String = body
        .iter()
        .map(stmt_to_kabootar)
        .collect::<Vec<_>>()
        .join("\n");
    if kabootar.is_empty() {
        return Ok(None);
    }
    let wrapped = format!("fn kv8_loop_fn() {{ {kabootar} }}");
    let compiled = crate::bytecode::compile_source(&wrapped)?;
    let Some(bc) = compiled.bytecode else {
        return Ok(None);
    };
    if !bc.uses_bytecode() {
        return Ok(None);
    }
    let func_def = bc
        .functions
        .iter()
        .find(|f| f.name == "kv8_loop_fn")
        .ok_or("loop body compile failed")?;
    Ok(Some(Kv8BytecodeFn {
        func: BytecodeFunction {
            def: Rc::new(func_def.clone()),
            closure: Environment::new(),
        },
    }))
}
