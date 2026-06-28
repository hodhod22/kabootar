//! Kv8 AST — JS-subset statements and expressions.

use super::context::Kv8Value;

/// Function parameter with optional default expression.
pub type Kv8Param = (String, Option<Expr>);

#[derive(Debug, Clone)]
pub struct Kv8Program {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    /// `var name` / `var name = init` — hoisted to function scope.
    Var(String, Expr),
    /// `let name = init` / `const name = init`
    Let(String, Expr),
    Assign(LValue, Expr),
    Expr(Expr),
    Return(Expr),
    If(Expr, Vec<Stmt>, Option<Vec<Stmt>>),
    For(String, Expr, Expr, Expr, Vec<Stmt>),
    While(Expr, Vec<Stmt>),
    /// `label: stmt` — targets `for` / `while` / `switch` / block.
    Label(String, Box<Stmt>),
    /// `{ ... }` statement block (e.g. labeled block).
    Block(Vec<Stmt>),
    /// `break` or `break label`
    Break(Option<String>),
    /// `continue` or `continue label`
    Continue(Option<String>),
    /// `do { } while (cond)`
    DoWhile(Vec<Stmt>, Expr),
    Throw(Expr),
    Switch(Expr, Vec<SwitchCase>, Option<Vec<Stmt>>),
    /// C-style `for (init; cond; update) { }`
    ForClassic(Vec<Stmt>, Option<Expr>, Option<Expr>, Vec<Stmt>),
    /// `for (lvalue in iterable) { }`
    ForIn(LValue, Expr, Vec<Stmt>),
    /// `for (lvalue of iterable) { }`
    ForOf(LValue, Expr, Vec<Stmt>),
    /// `import default from "mod"` / `import { a, b } from "mod"`
    Import {
        default: Option<String>,
        named: Vec<String>,
        from: String,
    },
    /// `export default expr`
    ExportDefault(Expr),
    /// `export { a, b }`
    ExportNamed(Vec<String>),
    TryCatch(Vec<Stmt>, Option<(String, Vec<Stmt>)>, Option<Vec<Stmt>>),
    Function(String, Vec<Kv8Param>, Vec<Stmt>),
    AsyncFunction(String, Vec<Kv8Param>, Vec<Stmt>),
}

#[derive(Debug, Clone)]
pub struct SwitchCase {
    pub label: Expr,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum LValue {
    Name(String),
    This,
    Member(Box<LValue>, String),
    Index(Box<LValue>, Box<Expr>),
    /// `(expr).field` assignment target
    MemberExpr(Box<Expr>, String),
    /// `(expr)[index]` assignment target
    IndexExpr(Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum ObjectEntryKey {
    Lit(String),
    Computed(Expr),
    Spread(Expr),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Lit(Kv8Value),
    Var(String),
    Member(Box<Expr>, String),
    Call(Box<Expr>, Vec<Expr>),
    Bin(Box<Expr>, char, Box<Expr>),
    Unary(char, Box<Expr>),
    Arrow(Vec<Kv8Param>, Box<Expr>),
    /// Block body for `() => { ... }`
    Block(Vec<Stmt>),
    /// `{ key: expr, ... }` object literal
    Object(Vec<(ObjectEntryKey, Expr)>),
    /// `[a, b, c]` array literal
    Array(Vec<Expr>),
    /// `base[index]` computed member / index
    Index(Box<Expr>, Box<Expr>),
    /// `function (a, b) { ... }` expression
    FunExpr(Vec<Kv8Param>, Vec<Stmt>),
    This,
    New(Box<Expr>, Vec<Expr>),
    Await(Box<Expr>),
    /// `a, b, c` — evaluates left-to-right, value is last operand
    Seq(Vec<Expr>),
    /// `lhs = rhs` / `lhs += rhs` assignment expression (right-associative)
    AssignExpr(LValue, char, Box<Expr>),
    /// `cond ? then : else`
    Cond(Box<Expr>, Box<Expr>, Box<Expr>),
    /// `++x` / `--x` (prefix) or `x++` / `x--` (postfix)
    Update(LValue, char, bool),
    /// `obj?.field`
    OptMember(Box<Expr>, String),
    /// `obj?.[index]`
    OptIndex(Box<Expr>, Box<Expr>),
    /// `obj?.(...args)`
    OptCall(Box<Expr>, Vec<Expr>),
    /// `` `lit${expr}` ``
    Template(Vec<TemplatePart>),
}

#[derive(Debug, Clone)]
pub enum TemplatePart {
    Lit(String),
    Expr(Box<Expr>),
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
