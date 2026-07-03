//! Compile Kv8 arrow/function bodies to Kabootar bytecode.

use super::ast::{Expr, Kv8Param, Kv8Program, ObjectEntryKey, Stmt};
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

pub fn compile_arrow(params: &[Kv8Param], body: &Expr) -> Result<Kv8BytecodeFn, String> {
    let param_names: Vec<&str> = params.iter().map(|(n, _)| n.as_str()).collect();
    let kabootar = format!(
        "fn kv8_arrow_fn({}) {{ return {} }}",
        param_names.join(", "),
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
        Stmt::Var(name, expr) => format!("var {name} = {};", expr_to_kabootar(expr)),
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
        Stmt::While(cond, body) => {
            let mut s = format!("while ({}) {{ ", expr_to_kabootar(cond));
            for st in body {
                s.push_str(&stmt_to_kabootar(st));
            }
            s.push_str(" }");
            s
        }
        Stmt::Break(l) => l
            .as_ref()
            .map(|n| format!("break {n};"))
            .unwrap_or_else(|| "break;".into()),
        Stmt::Continue(l) => l
            .as_ref()
            .map(|n| format!("continue {n};"))
            .unwrap_or_else(|| "continue;".into()),
        Stmt::Label(name, inner) => format!("{name}: {}", stmt_to_kabootar(inner)),
        Stmt::Block(stmts) => {
            let mut s = String::from("{ ");
            for st in stmts {
                s.push_str(&stmt_to_kabootar(st));
            }
            s.push('}');
            s
        }
        Stmt::DoWhile(body, cond) => {
            let mut s = String::from("do { ");
            for st in body {
                s.push_str(&stmt_to_kabootar(st));
            }
            s.push_str(&format!(" }} while ({})", expr_to_kabootar(cond)));
            s
        }
        Stmt::Throw(expr) => format!("throw {};", expr_to_kabootar(expr)),
        Stmt::Switch(disc, cases, default) => {
            let mut s = format!("switch ({}) {{ ", expr_to_kabootar(disc));
            for case in cases {
                s.push_str(&format!("case {}: ", expr_to_kabootar(&case.label)));
                for st in &case.body {
                    s.push_str(&stmt_to_kabootar(st));
                }
            }
            if let Some(def) = default {
                s.push_str("default: ");
                for st in def {
                    s.push_str(&stmt_to_kabootar(st));
                }
            }
            s.push_str(" }");
            s
        }
        Stmt::ForClassic(init, cond, update, body) => {
            let mut s = format!(
                "for ({}) {{ ",
                init.iter().map(stmt_to_kabootar).collect::<Vec<_>>().join("")
            );
            for st in body {
                s.push_str(&stmt_to_kabootar(st));
            }
            s.push_str(" }");
            let _ = (cond, update);
            s
        }
        Stmt::ForIn(lv, iter, body) => {
            let mut s = format!(
                "for ({} in {}) {{ ",
                lvalue_to_kabootar(lv),
                expr_to_kabootar(iter)
            );
            for st in body {
                s.push_str(&stmt_to_kabootar(st));
            }
            s.push_str(" }");
            s
        }
        Stmt::ForOf(lv, iter, body) => {
            let mut s = format!(
                "for ({} of {}) {{ ",
                lvalue_to_kabootar(lv),
                expr_to_kabootar(iter)
            );
            for st in body {
                s.push_str(&stmt_to_kabootar(st));
            }
            s.push_str(" }");
            s
        }
        Stmt::Import { default, named, from } => {
            let mut s = String::from("import ");
            if let Some(d) = default {
                s.push_str(d);
            }
            if !named.is_empty() {
                if default.is_some() {
                    s.push_str(", ");
                }
                s.push('{');
                s.push_str(&named.join(", "));
                s.push('}');
            }
            s.push_str(&format!(" from \"{from}\";"));
            s
        }
        Stmt::ExportDefault(e) => format!("export default {};", expr_to_kabootar(e)),
        Stmt::ExportNamed(names) => format!("export {{ {} }};", names.join(", ")),
        Stmt::TryCatch(try_b, catch, fin) => {
            let mut s = String::from("try { ");
            for st in try_b {
                s.push_str(&stmt_to_kabootar(st));
            }
            s.push_str(" }");
            if let Some((catch_var, catch_b)) = catch {
                s.push_str(&format!(" catch ({catch_var}) {{ "));
                for st in catch_b {
                    s.push_str(&stmt_to_kabootar(st));
                }
                s.push_str(" }");
            }
            if let Some(f) = fin {
                s.push_str(" finally { ");
                for st in f {
                    s.push_str(&stmt_to_kabootar(st));
                }
                s.push_str(" }");
            }
            s
        }
        Stmt::Function(name, params, body) => {
            let mut s = format!("fn {name}({}) {{ ", params_to_kabootar(params));
            for st in body {
                s.push_str(&stmt_to_kabootar(st));
            }
            s.push('}');
            s
        }
        Stmt::AsyncFunction(name, params, body) => {
            let mut s = format!("async fn {name}({}) {{ ", params_to_kabootar(params));
            for st in body {
                s.push_str(&stmt_to_kabootar(st));
            }
            s.push('}');
            s
        }
        Stmt::Assign(lv, expr) => format!("{} = {};", lvalue_to_kabootar(lv), expr_to_kabootar(expr)),
        Stmt::Class { name, .. } => format!("class {name} {{}}"),
    }
}

fn expr_to_kabootar(expr: &Expr) -> String {
    match expr {
        Expr::Lit(v) => super::ast::literal_to_string(v),
        Expr::Var(n) => n.clone(),
        Expr::Member(base, f) => format!("{}.{}", expr_to_kabootar(base), f),
        Expr::Index(base, i) => format!("{}[{}]", expr_to_kabootar(base), expr_to_kabootar(i)),
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
        Expr::Unary(op, inner) => match op {
            't' => format!("(typeof {})", expr_to_kabootar(inner)),
            'v' => format!("(void {})", expr_to_kabootar(inner)),
            _ => format!("({op}{})", expr_to_kabootar(inner)),
        },
        Expr::Seq(exprs) => format!(
            "({})",
            exprs
                .iter()
                .map(expr_to_kabootar)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expr::AssignExpr(lv, op, rhs) => {
            if *op == '=' {
                format!("{} = {}", lvalue_to_kabootar(lv), expr_to_kabootar(rhs))
            } else {
                format!(
                    "{} {} {}",
                    lvalue_to_kabootar(lv),
                    op,
                    expr_to_kabootar(rhs)
                )
            }
        }
        Expr::Cond(c, t, e) => format!(
            "({}) ? {} : {}",
            expr_to_kabootar(c),
            expr_to_kabootar(t),
            expr_to_kabootar(e)
        ),
        Expr::Update(lv, op, prefix) => {
            if *prefix {
                format!("{}{}", op, lvalue_to_kabootar(lv))
            } else {
                format!("{}{}", lvalue_to_kabootar(lv), op)
            }
        }
        Expr::Arrow(params, body) => {
            format!(
                "fn kv8_arrow_fn({}) {{ return {} }}",
                params_to_kabootar(params),
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
        Expr::Object(pairs) => format!(
            "{{ {} }}",
            pairs
                .iter()
                .map(|(k, e)| {
                    let key = match k {
                        ObjectEntryKey::Lit(s) => s.clone(),
                        ObjectEntryKey::Computed(expr) => format!("[{}]", expr_to_kabootar(expr)),
                        ObjectEntryKey::Spread(expr) => format!("...{}", expr_to_kabootar(expr)),
                    };
                    format!("{key}: {}", expr_to_kabootar(e))
                })
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expr::Array(elems) => format!(
            "[{}]",
            elems
                .iter()
                .map(expr_to_kabootar)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Expr::New(c, a) => format!(
            "new {}({})",
            expr_to_kabootar(c),
            a.iter().map(expr_to_kabootar).collect::<Vec<_>>().join(", ")
        ),
        Expr::Await(i) => format!("await {}", expr_to_kabootar(i)),
        Expr::OptMember(b, f) => format!("{}?.{}", expr_to_kabootar(b), f),
        Expr::OptIndex(b, i) => format!("{}?.[{}]", expr_to_kabootar(b), expr_to_kabootar(i)),
        Expr::OptCall(c, args) => format!(
            "{}?.({})",
            expr_to_kabootar(c),
            args.iter().map(expr_to_kabootar).collect::<Vec<_>>().join(", ")
        ),
        Expr::Template(parts) => format!(
            "`{}`",
            parts
                .iter()
                .map(|p| match p {
                    super::ast::TemplatePart::Lit(s) => s.clone(),
                    super::ast::TemplatePart::Expr(e) => format!("${{{}}}", expr_to_kabootar(e)),
                })
                .collect::<Vec<_>>()
                .join("")
        ),
        Expr::FunExpr(params, body) => {
            let mut s = format!("fn kv8_fun_expr({}) {{ ", params_to_kabootar(params));
            for st in body {
                s.push_str(&stmt_to_kabootar(st));
            }
            s.push('}');
            s
        }
        Expr::This => "this".into(),
    }
}

fn params_to_kabootar(params: &[Kv8Param]) -> String {
    params
        .iter()
        .map(|(n, _)| n.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn lvalue_to_kabootar(lv: &super::ast::LValue) -> String {
    match lv {
        super::ast::LValue::Name(n) => n.clone(),
        super::ast::LValue::This => "this".into(),
        super::ast::LValue::Member(base, f) => format!("{}.{}", lvalue_to_kabootar(base), f),
        super::ast::LValue::Index(base, i) => {
            format!("{}[{}]", lvalue_to_kabootar(base), expr_to_kabootar(i))
        }
        super::ast::LValue::MemberExpr(base, f) => {
            format!("({}).{}", expr_to_kabootar(base), f)
        }
        super::ast::LValue::IndexExpr(base, i) => {
            format!("({})[{}]", expr_to_kabootar(base), expr_to_kabootar(i))
        }
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
