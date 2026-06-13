//! Abstract syntax tree for Deific.

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    Str,
    Void,
    BigInt,
    List(Box<Type>),
    Dict(Box<Type>, Box<Type>),
    Set(Box<Type>),
    Tuple(Vec<Type>),
    TypeVar(String),
}

impl Type {
    pub fn cpp(&self) -> String {
        match self {
            Type::Int => "long long".into(),
            Type::Float => "double".into(),
            Type::Bool => "bool".into(),
            Type::Str => "std::string".into(),
            Type::Void => "void".into(),
            Type::BigInt => "deific::bigint".into(),
            Type::List(inner) => format!("std::vector<{}>", inner.cpp()),
            Type::Dict(k, v) => format!("std::unordered_map<{},{}>", k.cpp(), v.cpp()),
            Type::Set(inner) => format!("std::unordered_set<{}>", inner.cpp()),
            Type::Tuple(ts) => {
                let cs: Vec<String> = ts.iter().map(|t| t.cpp()).collect();
                format!("std::tuple<{}>", cs.join(","))
            }
            Type::TypeVar(name) => name.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<(String, Type)>,
}

#[derive(Debug, Clone)]
pub struct GlobalVar {
    pub name: String,
    pub ty: Option<Type>,
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub structs: Vec<StructDef>,
    pub globals: Vec<GlobalVar>,
    pub funcs: Vec<Func>,
}

#[derive(Debug, Clone)]
pub struct Func {
    pub name: String,
    pub type_params: Vec<String>,
    /// (param_name, is_ref, type)
    pub params: Vec<(String, bool, Type)>,
    pub ret: Type,
    pub body: Vec<Stmt>,
    #[allow(dead_code)]  // reserved for future tooling
    pub line: usize,
}

/// Statement with source line number (used for `#line` directives).
#[derive(Debug, Clone)]
pub struct Stmt {
    pub line: usize,
    pub kind: StmtKind,
}

impl Stmt {
    pub fn new(line: usize, kind: StmtKind) -> Self {
        Stmt { line, kind }
    }
}

#[derive(Debug, Clone)]
pub enum StmtKind {
    /// `targets [: ty] = value`.  Single element for plain assign, multiple for tuple unpack.
    Assign {
        targets: Vec<Expr>,
        ty: Option<Type>,
        value: Expr,
    },
    /// `target op= value`
    AugAssign {
        target: Expr,
        op: BinOp,
        value: Expr,
    },
    ExprStmt(Expr),
    Return(Option<Expr>),
    /// `for vars in iter:`  iter may be range(…) or any iterable.
    For {
        vars: Vec<String>,
        iter: Expr,
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
    Break,
    Continue,
    Pass,
    /// `global x, y` — marks names as referring to module-level globals.
    Global(Vec<String>),
    /// `defer expr()` — runs expr at end of enclosing function scope.
    Defer(Expr),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    None,
    Name(String),
    Tuple(Vec<Expr>),
    /// List literal `[a, b, c]`
    List(Vec<Expr>),
    /// `[elt for vars in iter if cond]`
    ListComp {
        elt: Box<Expr>,
        vars: Vec<String>,
        iter: Box<Expr>,
        cond: Option<Box<Expr>>,
    },
    /// `{k: v, ...}` or `{}`
    DictLiteral(Vec<(Expr, Expr)>),
    /// `{key: val for vars in iter if cond}`
    DictComp {
        key: Box<Expr>,
        val: Box<Expr>,
        vars: Vec<String>,
        iter: Box<Expr>,
        cond: Option<Box<Expr>>,
    },
    /// `{a, b, c}`
    SetLiteral(Vec<Expr>),
    /// `{elt for vars in iter if cond}`
    SetComp {
        elt: Box<Expr>,
        vars: Vec<String>,
        iter: Box<Expr>,
        cond: Option<Box<Expr>>,
    },
    Bin {
        op: BinOp,
        l: Box<Expr>,
        r: Box<Expr>,
    },
    Unary {
        op: UnOp,
        e: Box<Expr>,
    },
    /// Free function call: `f(a, b)`
    Call {
        func: String,
        args: Vec<Expr>,
    },
    /// Method call: `recv.name(args)`
    Method {
        recv: Box<Expr>,
        name: String,
        args: Vec<Expr>,
    },
    Index {
        base: Box<Expr>,
        idx: Box<Expr>,
    },
    /// `base[start:stop]` or `base[start:stop:step]`
    Slice {
        base: Box<Expr>,
        start: Option<Box<Expr>>,
        stop: Option<Box<Expr>>,
        step: Option<Box<Expr>>,
    },
    /// `*x` argument unpacking (only meaningful inside `print`)
    Star(Box<Expr>),
    /// `recv.field` (no call — field access on a struct)
    Field { recv: Box<Expr>, name: String },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, FloorDiv, Div, Mod, Pow,
    BitAnd, BitOr, BitXor, Shl, Shr,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
    In, NotIn,
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnOp {
    Neg,
    Not,
    BitNot,
}
