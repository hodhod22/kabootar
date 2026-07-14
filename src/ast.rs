#[derive(Debug, Clone, PartialEq)]
pub struct FnParam {
    pub name: String,
    pub type_ann: Option<KabType>,
    pub default: Option<Expr>,
}

/// Simple type annotation (v1: identifier only — `Number`, `String`, or type param `T`).
#[derive(Debug, Clone, PartialEq)]
pub enum KabType {
    Named(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Literal),
    Variable(String),
    Binary(Box<Expr>, BinaryOp, Box<Expr>),
    Unary(UnaryOp, Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    Function {
        name: String,
        type_params: Vec<String>,
        params: Vec<FnParam>,
        rest: Option<String>,
        return_type: Option<KabType>,
        body: Box<Expr>,
        public: bool,
        async_fn: bool,
        generator_fn: bool,
    },
    Arrow {
        params: Vec<FnParam>,
        rest: Option<String>,
        body: Box<Expr>,
        async_fn: bool,
        generator_fn: bool,
    },
    Yield(Box<Expr>),
    YieldStar(Box<Expr>),
    Await(Box<Expr>),
    Call {
        func: Box<Expr>,
        type_args: Vec<String>,
        args: Vec<CallArg>,
    },
    Block(Vec<Stmt>),
    Match(Box<Expr>, Vec<MatchArm>),
    While(Box<Expr>, Box<Expr>),
    WhileLet {
        pattern: Pattern,
        scrutinee: Box<Expr>,
        body: Box<Expr>,
    },
    DoWhile(Box<Expr>, Box<Expr>),
    ForEach(ForeachLoop),
    Switch {
        scrutinee: Box<Expr>,
        cases: Vec<SwitchCase>,
        default_body: Option<Box<Expr>>,
    },
    ForClassic {
        init: Option<Box<Stmt>>,
        cond: Option<Box<Expr>>,
        step: Option<Box<Expr>>,
        body: Box<Expr>,
    },
    TryCatch {
        body: Box<Expr>,
        err_name: String,
        handler: Box<Expr>,
        finally: Option<Box<Expr>>,
    },
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    IfLet {
        pattern: Pattern,
        scrutinee: Box<Expr>,
        body: Box<Expr>,
        else_branch: Option<Box<Expr>>,
    },
    Assign(AssignTarget, Box<Expr>),
    Member(Box<Expr>, String, Vec<String>),
    Index(Box<Expr>, Box<Expr>),
    OptionalMember(Box<Expr>, String),
    OptionalIndex(Box<Expr>, Box<Expr>),
    OptionalCall(Box<Expr>, Vec<CallArg>),
    This,
    Super,
    Break,
    Fallthrough,
    Continue,
    Pass,
    Assert {
        condition: Box<Expr>,
        message: Option<Box<Expr>>,
    },
    With {
        name: String,
        value: Box<Expr>,
        body: Box<Expr>,
    },
    ImportMeta,
    DynamicImport(Box<Expr>),
    ResultQuestion(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum CallArg {
    Expr(Expr),
    Spread(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Not,
    Neg,
    BitNot,
    Delete,
    Throw,
    Raise,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssignTarget {
    Name(String),
    Member(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    Pattern(BindingPattern),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BindingPattern {
    Name(String),
    Wildcard,
    Array(Vec<BindingPattern>),
    Object(Vec<ObjectBind>),
    Rest(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObjectBind {
    Shorthand(String),
    Field { key: String, pattern: BindingPattern },
    Rest(String),
}

/// Names introduced by a binding pattern (for `pub let` exports).
pub fn exported_binding_names(pattern: &BindingPattern) -> Vec<String> {
    let mut names = Vec::new();
    collect_binding_names(pattern, &mut names);
    names
}

fn collect_binding_names(pattern: &BindingPattern, out: &mut Vec<String>) {
    match pattern {
        BindingPattern::Name(name) => out.push(name.clone()),
        BindingPattern::Wildcard => {}
        BindingPattern::Rest(name) => {
            if !name.is_empty() {
                out.push(name.clone());
            }
        }
        BindingPattern::Array(items) => {
            for item in items {
                collect_binding_names(item, out);
            }
        }
        BindingPattern::Object(fields) => {
            for field in fields {
                match field {
                    ObjectBind::Shorthand(key) => out.push(key.clone()),
                    ObjectBind::Field { pattern, .. } => collect_binding_names(pattern, out),
                    ObjectBind::Rest(name) => {
                        if !name.is_empty() {
                            out.push(name.clone());
                        }
                    }
                }
            }
        }
    }
}

pub fn fn_param_names(params: &[FnParam]) -> Vec<String> {
    params.iter().map(|p| p.name.clone()).collect()
}

impl Expr {
    pub fn call(func: Expr, args: Vec<CallArg>) -> Self {
        Expr::Call {
            func: Box::new(func),
            type_args: Vec::new(),
            args,
        }
    }
}

pub fn fn_param_defaults(params: &[FnParam]) -> Vec<Option<Expr>> {
    params.iter().map(|p| p.default.clone()).collect()
}

pub fn fn_has_defaults_or_rest(params: &[FnParam], rest: &Option<String>) -> bool {
    rest.is_some() || params.iter().any(|p| p.default.is_some())
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassField {
    pub name: String,
    pub type_name: String,
    pub default: Option<Expr>,
    pub private: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassMethod {
    pub name: String,
    pub type_params: Vec<String>,
    pub params: Vec<String>,
    pub body: Expr,
    pub private: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceMethod {
    pub name: String,
    pub params: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForeachLoop {
    pub var: String,
    pub iterable: Box<Expr>,
    pub body: Box<Expr>,
    pub async_for: bool,
    /// `true` for `for x of xs` (values); `false` for `for i in xs` (indices/keys).
    pub by_value: bool,
    pub immutable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase {
    pub value: Expr,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let {
        pattern: BindingPattern,
        init: Option<Expr>,
        public: bool,
        immutable: bool,
    },
    Interface {
        name: String,
        methods: Vec<InterfaceMethod>,
    },
    Enum {
        name: String,
        type_params: Vec<String>,
        variants: Vec<EnumVariant>,
    },
    Class {
        name: String,
        type_params: Vec<String>,
        extends: Option<String>,
        extends_type_args: Vec<String>,
        implements: Vec<String>,
        fields: Vec<ClassField>,
        methods: Vec<ClassMethod>,
    },
    Import {
        module: String,
        public: bool,
    },
    Using {
        name: String,
        init: Expr,
    },
    Expr(Expr),
    Return(Option<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number(i64),
    Float(f64),
    BigInt(String),
    String(String),
    Bool(bool),
    Null,
    Undefined,
    Nan,
    Some(Box<Expr>),
    None,
    Ok(Box<Expr>),
    Err(Box<Expr>),
    Array(Vec<ArrayPiece>),
    Object(Vec<ObjectPiece>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArrayPiece {
    Item(Expr),
    Spread(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObjectPiece {
    Field { key: String, value: Expr },
    Method {
        key: String,
        params: Vec<FnParam>,
        rest: Option<String>,
        body: Box<Expr>,
        async_fn: bool,
    },
    Spread(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    NullishCoalesce,
    In,
    Is,
    IsNot,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Ushr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Wildcard,
    Number(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Null,
    Undefined,
    Nan,
    Variable(String),
    Some(Box<Pattern>),
    None,
    Ok(Box<Pattern>),
    Err(Box<Pattern>),
    EnumVariant {
        enum_name: String,
        variant: String,
        fields: Vec<Pattern>,
    },
    Array(Vec<PatternPiece>),
    Object(Vec<PatternField>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternPiece {
    Item(Pattern),
    Wildcard,
    Rest(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternField {
    Shorthand(String),
    Field { key: String, pattern: Pattern },
    Rest(String),
}
