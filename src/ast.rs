//! Abstract syntax tree for Deific.
//!
//! The tree is deliberately small: this is the v0 vertical slice that proves
//! the thesis (Python syntax -> C++ -> native binary). Locals lean on C++
//! `auto`, so the AST tracks types only where C++ genuinely needs them:
//! function signatures.

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    Str,
    Void,
    List(Box<Type>),
}

impl Type {
    /// The C++ spelling of this type. `int` is 64-bit by design (see README).
    pub fn cpp(&self) -> String {
        match self {
            Type::Int => "long long".into(),
            Type::Float => "double".into(),
            Type::Bool => "bool".into(),
            Type::Str => "std::string".into(),
            Type::Void => "void".into(),
            Type::List(inner) => format!("std::vector<{}>", inner.cpp()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Program {
    pub funcs: Vec<Func>,
}

#[derive(Debug, Clone)]
pub struct Func {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub ret: Type,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    /// `target [: ty] = value`. The emitter decides declare-vs-reassign by
    /// tracking which names are already in scope.
    Assign {
        target: Expr,
        ty: Option<Type>,
        value: Expr,
    },
    ExprStmt(Expr),
    Return(Option<Expr>),
    /// `for <var> in range(<count>):`
    For {
        var: String,
        count: Expr,
        body: Vec<Stmt>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    If {
        cond: Expr,
        then: Vec<Stmt>,
        elifs: Vec<(Expr, Vec<Stmt>)>,
        els: Option<Vec<Stmt>>,
    },
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    Str(String),
    Bool(bool),
    Name(String),
    Bin {
        op: BinOp,
        l: Box<Expr>,
        r: Box<Expr>,
    },
    Unary {
        op: UnOp,
        e: Box<Expr>,
    },
    /// Free function call: `f(a, b)`.
    Call {
        func: String,
        args: Vec<Expr>,
    },
    /// Method call: `recv.name(args)`.
    Method {
        recv: Box<Expr>,
        name: String,
        args: Vec<Expr>,
    },
    Index {
        base: Box<Expr>,
        idx: Box<Expr>,
    },
    /// `*x` argument unpacking (only meaningful inside `print`).
    Star(Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    FloorDiv,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnOp {
    Neg,
    Not,
}
