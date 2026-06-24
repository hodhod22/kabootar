//! Kv8 AST — JS-subset statements and expressions.

use super::context::Kv8Value;

#[derive(Debug, Clone)]
pub struct Kv8Program {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let(String, Expr),
    Assign(LValue, Expr),
    Expr(Expr),
    Return(Expr),
    If(Expr, Vec<Stmt>, Option<Vec<Stmt>>),
    For(String, Expr, Expr, Expr, Vec<Stmt>),
    Function(String, Vec<String>, Vec<Stmt>),
}

#[derive(Debug, Clone)]
pub enum LValue {
    Name(String),
    Member(Box<LValue>, String),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Lit(Kv8Value),
    Var(String),
    Member(Box<Expr>, String),
    Call(Box<Expr>, Vec<Expr>),
    Bin(Box<Expr>, char, Box<Expr>),
    Unary(char, Box<Expr>),
    Arrow(Vec<String>, Box<Expr>),
    /// Block body for `() => { ... }`
    Block(Vec<Stmt>),
}

pub fn literal_to_string(v: &Kv8Value) -> String {
    match v {
        Kv8Value::Str(s) => format!("\"{}\"", s.replace('"', "\\\"")),
        Kv8Value::Num(n) => {
            if n.fract() == 0.0 {
                format!("{}", *n as i64)
            } else {
                n.to_string()
            }
        }
        Kv8Value::Bool(b) => if *b { "true" } else { "false" }.into(),
        Kv8Value::Null => "null".into(),
        _ => "null".into(),
    }
}
