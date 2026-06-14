//! Recursive-descent parser with Pratt expression layer.
//!
//! Operator precedence (highest → lowest):
//!   postfix  []  .m()  call
//!   **       (right-assoc)
//!   unary    -  ~
//!   *  /  //  %
//!   +  -
//!   <<  >>
//!   &
//!   ^
//!   |
//!   in  not in  ==  !=  <  <=  >  >=   (non-chained)
//!   not
//!   and
//!   or

use crate::ast::*;
use crate::lexer::{is_keyword, Tok};

#[derive(Debug)]
pub struct ParseError {
    pub line: usize,
    pub msg: String,
}

pub struct Parser {
    toks: Vec<(Tok, usize)>,
    pos: usize,
}

type PResult<T> = Result<T, ParseError>;

impl Parser {
    pub fn new(toks: Vec<(Tok, usize)>) -> Self {
        Parser { toks, pos: 0 }
    }

    fn peek(&self) -> &Tok {
        &self.toks[self.pos].0
    }
    fn peek2(&self) -> &Tok {
        let next = (self.pos + 1).min(self.toks.len() - 1);
        &self.toks[next].0
    }
    fn line(&self) -> usize {
        self.toks[self.pos].1
    }
    fn bump(&mut self) -> Tok {
        let t = self.toks[self.pos].0.clone();
        if self.pos + 1 < self.toks.len() {
            self.pos += 1;
        }
        t
    }
    fn err<T>(&self, msg: impl Into<String>) -> PResult<T> {
        Err(ParseError { line: self.line(), msg: msg.into() })
    }

    fn eat_op(&mut self, op: &str) -> PResult<()> {
        if matches!(self.peek(), Tok::Op(o) if o == op) {
            self.bump();
            Ok(())
        } else {
            self.err(format!("expected '{}', found {:?}", op, self.peek()))
        }
    }
    fn is_op(&self, op: &str) -> bool {
        matches!(self.peek(), Tok::Op(o) if o == op)
    }
    fn is_kw(&self, kw: &str) -> bool {
        matches!(self.peek(), Tok::Ident(w) if w == kw)
    }
    fn eat_kw(&mut self, kw: &str) -> PResult<()> {
        if self.is_kw(kw) {
            self.bump();
            Ok(())
        } else {
            self.err(format!("expected '{}', found {:?}", kw, self.peek()))
        }
    }
    fn ident(&mut self) -> PResult<String> {
        match self.peek().clone() {
            Tok::Ident(w) if !is_keyword(&w) => { self.bump(); Ok(w) }
            other => self.err(format!("expected identifier, found {:?}", other)),
        }
    }
    fn skip_newlines(&mut self) {
        while matches!(self.peek(), Tok::Newline) {
            self.bump();
        }
    }

    // ---- program ------------------------------------------------------------

    pub fn parse_program(&mut self) -> PResult<Program> {
        let mut imports = Vec::new();
        let mut structs = Vec::new();
        let mut globals = Vec::new();
        let mut funcs = Vec::new();
        self.skip_newlines();
        while !matches!(self.peek(), Tok::Eof) {
            if self.is_kw("bring") {
                self.bump();
                // Allow dotted paths: bring math.utils → "math/utils"
                let mut path = self.ident()?;
                while self.is_op(".") {
                    self.bump();
                    path.push('/');
                    path.push_str(&self.ident()?);
                }
                self.expect_newline()?;
                imports.push(path);
                self.skip_newlines();
                continue;
            }
            if self.is_kw("struct") {
                structs.push(self.parse_struct()?);
            } else if self.is_kw("func") || self.is_kw("inline") {
                funcs.push(self.parse_func()?);
            } else {
                // Top-level variable: `name [: type] = expr`
                let name = self.ident()?;
                let ty = if self.is_op(":") {
                    self.bump();
                    Some(self.parse_type()?)
                } else {
                    None
                };
                self.eat_op("=")?;
                let value = self.parse_rhs()?;
                self.expect_newline()?;
                globals.push(crate::ast::GlobalVar { name, ty, value });
            }
            self.skip_newlines();
        }
        Ok(Program { imports, structs, globals, funcs })
    }

    fn parse_struct(&mut self) -> PResult<crate::ast::StructDef> {
        self.eat_kw("struct")?;
        let name = self.ident()?;
        self.eat_op(":")?;
        // Indented block of `field: type` lines
        if !matches!(self.peek(), Tok::Newline) {
            return self.err("expected newline after struct name");
        }
        self.bump();
        if !matches!(self.peek(), Tok::Indent) {
            return self.err("expected indented field list");
        }
        self.bump();
        let mut fields = Vec::new();
        while !matches!(self.peek(), Tok::Dedent | Tok::Eof) {
            self.skip_newlines();
            if matches!(self.peek(), Tok::Dedent | Tok::Eof) { break; }
            let fname = self.ident()?;
            self.eat_op(":")?;
            let ftype = self.parse_type()?;
            self.expect_newline()?;
            fields.push((fname, ftype));
        }
        if matches!(self.peek(), Tok::Dedent) { self.bump(); }
        Ok(crate::ast::StructDef { name, fields })
    }

    // ---- functions ----------------------------------------------------------

    fn parse_func(&mut self) -> PResult<Func> {
        let line = self.line();
        let is_inline = if self.is_kw("inline") { self.bump(); true } else { false };
        self.eat_kw("func")?;
        let name = self.ident()?;

        // Optional type parameters: [T, U, ...]
        let type_params = if self.is_op("[") {
            self.bump();
            let mut tps = vec![self.ident()?];
            while self.is_op(",") {
                self.bump();
                if self.is_op("]") { break; }
                tps.push(self.ident()?);
            }
            self.eat_op("]")?;
            tps
        } else {
            vec![]
        };

        self.eat_op("(")?;
        let mut params = Vec::new();
        if !self.is_op(")") {
            loop {
                let pname = self.ident()?;
                self.eat_op(":")?;
                let is_ref = if self.is_kw("ref") { self.bump(); true } else { false };
                let ty = self.parse_type()?;
                params.push((pname, is_ref, ty));
                if self.is_op(",") { self.bump(); } else { break; }
            }
        }
        self.eat_op(")")?;

        let ret = if self.is_op("->") {
            self.bump();
            self.parse_type()?
        } else {
            Type::Void
        };
        self.eat_op(":")?;
        let body = self.parse_block()?;
        Ok(Func { name, is_inline, type_params, params, ret, body, line })
    }

    fn parse_type(&mut self) -> PResult<Type> {
        // `ref` used inside parameter lists is handled before calling parse_type
        if self.is_kw("bigint") { self.bump(); return Ok(Type::BigInt); }

        // Go-style tuple return: `(int, bool)`
        if self.is_op("(") {
            self.bump();
            let mut ts = vec![self.parse_type()?];
            while self.is_op(",") {
                self.bump();
                if self.is_op(")") { break; }
                ts.push(self.parse_type()?);
            }
            self.eat_op(")")?;
            return Ok(Type::Tuple(ts));
        }

        match self.peek().clone() {
            Tok::Ident(name) => {
                self.bump();
                match name.as_str() {
                    "int"   => Ok(Type::Int),
                    "float" => Ok(Type::Float),
                    "bool"  => Ok(Type::Bool),
                    "str"   => Ok(Type::Str),
                    "void"  => Ok(Type::Void),
                    "list"  => {
                        self.eat_op("[")?;
                        let inner = self.parse_type()?;
                        self.eat_op("]")?;
                        Ok(Type::List(Box::new(inner)))
                    }
                    "dict"  => {
                        self.eat_op("[")?;
                        let k = self.parse_type()?;
                        self.eat_op(",")?;
                        let v = self.parse_type()?;
                        self.eat_op("]")?;
                        Ok(Type::Dict(Box::new(k), Box::new(v)))
                    }
                    "set"   => {
                        self.eat_op("[")?;
                        let inner = self.parse_type()?;
                        self.eat_op("]")?;
                        Ok(Type::Set(Box::new(inner)))
                    }
                    "tuple" => {
                        self.eat_op("[")?;
                        let mut ts = vec![self.parse_type()?];
                        while self.is_op(",") {
                            self.bump();
                            if self.is_op("]") { break; }
                            ts.push(self.parse_type()?);
                        }
                        self.eat_op("]")?;
                        Ok(Type::Tuple(ts))
                    }
                    other if !is_keyword(other) => Ok(Type::TypeVar(other.to_string())),
                    other => self.err(format!("unknown type '{}'", other)),
                }
            }
            other => self.err(format!("expected type, found {:?}", other)),
        }
    }

    fn parse_block(&mut self) -> PResult<Vec<Stmt>> {
        if !matches!(self.peek(), Tok::Newline) {
            return self.err("expected newline before indented block");
        }
        self.bump();
        if !matches!(self.peek(), Tok::Indent) {
            return self.err("expected an indented block");
        }
        self.bump();
        let mut stmts = Vec::new();
        while !matches!(self.peek(), Tok::Dedent | Tok::Eof) {
            self.skip_newlines();
            if matches!(self.peek(), Tok::Dedent | Tok::Eof) { break; }
            stmts.push(self.parse_stmt()?);
        }
        if matches!(self.peek(), Tok::Dedent) {
            self.bump();
        }
        Ok(stmts)
    }

    // ---- statements ---------------------------------------------------------

    fn parse_stmt(&mut self) -> PResult<Stmt> {
        let line = self.line();

        macro_rules! stmt {
            ($kind:expr) => { Stmt::new(line, $kind) };
        }

        if self.is_kw("return") {
            self.bump();
            if matches!(self.peek(), Tok::Newline | Tok::Eof | Tok::Dedent) {
                self.expect_newline()?;
                return Ok(stmt!(StmtKind::Return(None)));
            }
            let e = self.parse_rhs()?;
            self.expect_newline()?;
            return Ok(stmt!(StmtKind::Return(Some(e))));
        }
        if self.is_kw("break") {
            self.bump(); self.expect_newline()?;
            return Ok(stmt!(StmtKind::Break));
        }
        if self.is_kw("continue") {
            self.bump(); self.expect_newline()?;
            return Ok(stmt!(StmtKind::Continue));
        }
        if self.is_kw("pass") {
            self.bump(); self.expect_newline()?;
            return Ok(stmt!(StmtKind::Pass));
        }
        if self.is_kw("global") {
            self.bump();
            let mut names = vec![self.ident()?];
            while self.is_op(",") {
                self.bump();
                names.push(self.ident()?);
            }
            self.expect_newline()?;
            return Ok(stmt!(StmtKind::Global(names)));
        }
        if self.is_kw("defer") {
            self.bump();
            let e = self.parse_expr()?;
            self.expect_newline()?;
            return Ok(stmt!(StmtKind::Defer(e)));
        }
        if self.is_kw("const") {
            self.bump();
            let name = self.ident()?;
            self.eat_op("=")?;
            let value = self.parse_rhs()?;
            self.expect_newline()?;
            return Ok(stmt!(StmtKind::Const { name, value }));
        }
        if self.is_kw("static") {
            self.bump();
            let name = self.ident()?;
            let ty = if self.is_op(":") { self.bump(); Some(self.parse_type()?) } else { None };
            self.eat_op("=")?;
            let value = self.parse_rhs()?;
            self.expect_newline()?;
            return Ok(stmt!(StmtKind::Static { name, ty, value }));
        }
        if self.is_kw("for") {
            return self.parse_for();
        }
        if self.is_kw("while") {
            self.bump();
            let cond = self.parse_expr()?;
            self.eat_op(":")?;
            let body = self.parse_block()?;
            return Ok(stmt!(StmtKind::While { cond, body }));
        }
        if self.is_kw("if") {
            return self.parse_if();
        }

        // assignment, augmented assignment, or bare expression
        let first = self.parse_or()?;

        // Multi-target: `a, b = ...`
        if self.is_op(",") {
            let mut targets = vec![first];
            while self.is_op(",") {
                self.bump();
                // Allow trailing comma before `=`
                if self.is_op("=") { break; }
                targets.push(self.parse_or()?);
            }
            self.eat_op("=")?;
            let value = self.parse_rhs()?;
            self.expect_newline()?;
            return Ok(stmt!(StmtKind::Assign { targets, ty: None, value }));
        }

        // Augmented assignment: `x op= expr`
        if let Some(op) = self.peek_aug_op() {
            self.bump();
            let value = self.parse_expr()?;
            self.expect_newline()?;
            return Ok(stmt!(StmtKind::AugAssign { target: first, op, value }));
        }

        // Type-annotated assign: `x: ty = expr`
        if self.is_op(":") {
            self.bump();
            let ty = self.parse_type()?;
            self.eat_op("=")?;
            let value = self.parse_rhs()?;
            self.expect_newline()?;
            return Ok(stmt!(StmtKind::Assign { targets: vec![first], ty: Some(ty), value }));
        }

        // Plain assign: `target = expr`
        if self.is_op("=") {
            self.bump();
            let value = self.parse_rhs()?;
            self.expect_newline()?;
            return Ok(stmt!(StmtKind::Assign { targets: vec![first], ty: None, value }));
        }

        self.expect_newline()?;
        Ok(stmt!(StmtKind::ExprStmt(first)))
    }

    /// Parse RHS which may be a bare tuple: `a, b, c`
    fn parse_rhs(&mut self) -> PResult<Expr> {
        let first = self.parse_or()?;
        if self.is_op(",") {
            let mut elts = vec![first];
            while self.is_op(",") {
                self.bump();
                if matches!(self.peek(), Tok::Newline | Tok::Eof | Tok::Dedent) { break; }
                elts.push(self.parse_or()?);
            }
            return Ok(Expr::Tuple(elts));
        }
        Ok(first)
    }

    fn peek_aug_op(&self) -> Option<BinOp> {
        match self.peek() {
            Tok::Op(o) => match o.as_str() {
                "+=" => Some(BinOp::Add), "-=" => Some(BinOp::Sub), "*=" => Some(BinOp::Mul),
                "/=" => Some(BinOp::Div), "//=" => Some(BinOp::FloorDiv), "%=" => Some(BinOp::Mod),
                "**=" => Some(BinOp::Pow), "&=" => Some(BinOp::BitAnd), "|=" => Some(BinOp::BitOr),
                "^=" => Some(BinOp::BitXor), "<<=" => Some(BinOp::Shl), ">>=" => Some(BinOp::Shr),
                _ => None,
            },
            _ => None,
        }
    }

    fn parse_for(&mut self) -> PResult<Stmt> {
        let line = self.line();
        self.eat_kw("for")?;
        let vars = self.parse_for_vars()?;
        self.eat_kw("in")?;
        let iter = self.parse_or()?;
        self.eat_op(":")?;
        let body = self.parse_block()?;
        Ok(Stmt::new(line, StmtKind::For { vars, iter, body }))
    }

    fn parse_for_vars(&mut self) -> PResult<Vec<String>> {
        let mut vars = vec![self.ident()?];
        while self.is_op(",") {
            self.bump();
            // Stop if next is `in` keyword
            if self.is_kw("in") { break; }
            vars.push(self.ident()?);
        }
        Ok(vars)
    }

    fn parse_if(&mut self) -> PResult<Stmt> {
        let line = self.line();
        self.eat_kw("if")?;
        let cond = self.parse_expr()?;
        self.eat_op(":")?;
        let then = self.parse_block()?;
        let mut elifs = Vec::new();
        let mut els = None;
        loop {
            self.skip_newlines();
            if self.is_kw("elif") {
                self.bump();
                let c = self.parse_expr()?;
                self.eat_op(":")?;
                let b = self.parse_block()?;
                elifs.push((c, b));
            } else if self.is_kw("else") {
                self.bump();
                self.eat_op(":")?;
                els = Some(self.parse_block()?);
                break;
            } else {
                break;
            }
        }
        Ok(Stmt::new(line, StmtKind::If { cond, then, elifs, els }))
    }

    fn expect_newline(&mut self) -> PResult<()> {
        match self.peek() {
            Tok::Newline => { self.bump(); Ok(()) }
            Tok::Eof | Tok::Dedent => Ok(()),
            other => self.err(format!("expected end of line, found {:?}", other)),
        }
    }

    // ---- expressions --------------------------------------------------------

    /// Top-level expression (does NOT consume bare commas as tuple; use parse_rhs for that).
    pub fn parse_expr(&mut self) -> PResult<Expr> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> PResult<Expr> {
        let mut l = self.parse_and()?;
        while self.is_kw("or") {
            self.bump();
            let r = self.parse_and()?;
            l = Expr::Bin { op: BinOp::Or, l: Box::new(l), r: Box::new(r) };
        }
        Ok(l)
    }

    fn parse_and(&mut self) -> PResult<Expr> {
        let mut l = self.parse_not()?;
        while self.is_kw("and") {
            self.bump();
            let r = self.parse_not()?;
            l = Expr::Bin { op: BinOp::And, l: Box::new(l), r: Box::new(r) };
        }
        Ok(l)
    }

    fn parse_not(&mut self) -> PResult<Expr> {
        if self.is_kw("not") {
            // Distinguish `not in` (comparison, handled in parse_cmp) from unary `not`.
            // At this point, if peek is `not` and peek2 is `in`, it means we arrived here
            // from parse_and without a left operand — that would be `not (x in y)` written
            // as `not x in y`. We let parse_cmp handle the `in` part normally.
            self.bump();
            let e = self.parse_not()?;
            return Ok(Expr::Unary { op: UnOp::Not, e: Box::new(e) });
        }
        self.parse_cmp()
    }

    fn parse_cmp(&mut self) -> PResult<Expr> {
        let l = self.parse_bitor()?;
        // Check for comparison / membership operators
        let op = match self.peek() {
            Tok::Op(o) => match o.as_str() {
                "==" => Some(BinOp::Eq), "!=" => Some(BinOp::Ne),
                "<"  => Some(BinOp::Lt), "<=" => Some(BinOp::Le),
                ">"  => Some(BinOp::Gt), ">=" => Some(BinOp::Ge),
                _ => None,
            },
            Tok::Ident(w) if w == "in" => Some(BinOp::In),
            Tok::Ident(w) if w == "not" => {
                // `not in` — peek ahead
                if matches!(self.peek2(), Tok::Ident(w2) if w2 == "in") {
                    Some(BinOp::NotIn)
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(op) = op {
            if op == BinOp::NotIn {
                self.bump(); // consume `not`
                self.bump(); // consume `in`
            } else {
                self.bump();
            }
            let r = self.parse_bitor()?;
            return Ok(Expr::Bin { op, l: Box::new(l), r: Box::new(r) });
        }
        Ok(l)
    }

    fn parse_bitor(&mut self) -> PResult<Expr> {
        let mut l = self.parse_bitxor()?;
        while self.is_op("|") {
            self.bump();
            let r = self.parse_bitxor()?;
            l = Expr::Bin { op: BinOp::BitOr, l: Box::new(l), r: Box::new(r) };
        }
        Ok(l)
    }

    fn parse_bitxor(&mut self) -> PResult<Expr> {
        let mut l = self.parse_bitand()?;
        while self.is_op("^") {
            self.bump();
            let r = self.parse_bitand()?;
            l = Expr::Bin { op: BinOp::BitXor, l: Box::new(l), r: Box::new(r) };
        }
        Ok(l)
    }

    fn parse_bitand(&mut self) -> PResult<Expr> {
        let mut l = self.parse_shift()?;
        while self.is_op("&") {
            self.bump();
            let r = self.parse_shift()?;
            l = Expr::Bin { op: BinOp::BitAnd, l: Box::new(l), r: Box::new(r) };
        }
        Ok(l)
    }

    fn parse_shift(&mut self) -> PResult<Expr> {
        let mut l = self.parse_add()?;
        loop {
            let op = match self.peek() {
                Tok::Op(o) if o == "<<" => BinOp::Shl,
                Tok::Op(o) if o == ">>" => BinOp::Shr,
                _ => break,
            };
            self.bump();
            let r = self.parse_add()?;
            l = Expr::Bin { op, l: Box::new(l), r: Box::new(r) };
        }
        Ok(l)
    }

    fn parse_add(&mut self) -> PResult<Expr> {
        let mut l = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                Tok::Op(o) if o == "+" => BinOp::Add,
                Tok::Op(o) if o == "-" => BinOp::Sub,
                _ => break,
            };
            self.bump();
            let r = self.parse_mul()?;
            l = Expr::Bin { op, l: Box::new(l), r: Box::new(r) };
        }
        Ok(l)
    }

    fn parse_mul(&mut self) -> PResult<Expr> {
        let mut l = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Tok::Op(o) if o == "*"  => BinOp::Mul,
                Tok::Op(o) if o == "/"  => BinOp::Div,
                Tok::Op(o) if o == "//" => BinOp::FloorDiv,
                Tok::Op(o) if o == "%"  => BinOp::Mod,
                _ => break,
            };
            self.bump();
            let r = self.parse_unary()?;
            l = Expr::Bin { op, l: Box::new(l), r: Box::new(r) };
        }
        Ok(l)
    }

    fn parse_unary(&mut self) -> PResult<Expr> {
        if self.is_op("-") {
            self.bump();
            let e = self.parse_unary()?;
            return Ok(Expr::Unary { op: UnOp::Neg, e: Box::new(e) });
        }
        if self.is_op("~") {
            self.bump();
            let e = self.parse_unary()?;
            return Ok(Expr::Unary { op: UnOp::BitNot, e: Box::new(e) });
        }
        if self.is_op("*") {
            self.bump();
            let e = self.parse_unary()?;
            return Ok(Expr::Star(Box::new(e)));
        }
        self.parse_pow()
    }

    fn parse_pow(&mut self) -> PResult<Expr> {
        let base = self.parse_postfix()?;
        if self.is_op("**") {
            self.bump();
            // Right-associative: recurse into parse_unary so `-x**2` = `-(x**2)`
            let exp = self.parse_unary()?;
            return Ok(Expr::Bin { op: BinOp::Pow, l: Box::new(base), r: Box::new(exp) });
        }
        Ok(base)
    }

    fn parse_postfix(&mut self) -> PResult<Expr> {
        let mut e = self.parse_primary()?;
        loop {
            if self.is_op("[") {
                self.bump();
                // Slice: `[start:stop]` or `[start:stop:step]`
                if self.is_op(":") {
                    // `[:stop]` or `[:]`
                    self.bump();
                    let stop = if !self.is_op(":") && !self.is_op("]") {
                        Some(Box::new(self.parse_expr()?))
                    } else { None };
                    let step = if self.is_op(":") {
                        self.bump();
                        if !self.is_op("]") { Some(Box::new(self.parse_expr()?)) } else { None }
                    } else { None };
                    self.eat_op("]")?;
                    e = Expr::Slice { base: Box::new(e), start: None, stop, step };
                } else {
                    let idx = self.parse_expr()?;
                    if self.is_op(":") {
                        // `[start:...]`
                        self.bump();
                        let stop = if !self.is_op(":") && !self.is_op("]") {
                            Some(Box::new(self.parse_expr()?))
                        } else { None };
                        let step = if self.is_op(":") {
                            self.bump();
                            if !self.is_op("]") { Some(Box::new(self.parse_expr()?)) } else { None }
                        } else { None };
                        self.eat_op("]")?;
                        e = Expr::Slice { base: Box::new(e), start: Some(Box::new(idx)), stop, step };
                    } else {
                        self.eat_op("]")?;
                        e = Expr::Index { base: Box::new(e), idx: Box::new(idx) };
                    }
                }
            } else if self.is_op(".") {
                self.bump();
                let name = self.ident()?;
                if self.is_op("(") {
                    self.bump();
                    let args = self.parse_args()?;
                    self.eat_op(")")?;
                    e = Expr::Method { recv: Box::new(e), name, args };
                } else {
                    e = Expr::Field { recv: Box::new(e), name };
                }
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn parse_args(&mut self) -> PResult<Vec<Expr>> {
        let mut args = Vec::new();
        if self.is_op(")") { return Ok(args); }
        loop {
            args.push(self.parse_expr()?);
            if self.is_op(",") { self.bump(); } else { break; }
        }
        Ok(args)
    }

    fn parse_primary(&mut self) -> PResult<Expr> {
        match self.peek().clone() {
            Tok::Int(v)   => { self.bump(); Ok(Expr::Int(v)) }
            Tok::Float(v) => { self.bump(); Ok(Expr::Float(v)) }
            Tok::Str(s)   => { self.bump(); Ok(Expr::Str(s)) }
            Tok::Ident(w) if w == "True"  => { self.bump(); Ok(Expr::Bool(true)) }
            Tok::Ident(w) if w == "False" => { self.bump(); Ok(Expr::Bool(false)) }
            Tok::Ident(w) if w == "None"  => { self.bump(); Ok(Expr::None) }

            // Parenthesised expression or tuple
            Tok::Op(o) if o == "(" => {
                self.bump();
                if self.is_op(")") {
                    self.bump();
                    return Ok(Expr::Tuple(vec![]));
                }
                let first = self.parse_expr()?;
                if self.is_op(",") {
                    let mut elts = vec![first];
                    while self.is_op(",") {
                        self.bump();
                        if self.is_op(")") { break; }
                        elts.push(self.parse_expr()?);
                    }
                    self.eat_op(")")?;
                    Ok(Expr::Tuple(elts))
                } else {
                    self.eat_op(")")?;
                    Ok(first)
                }
            }

            // List literal or list comprehension
            Tok::Op(o) if o == "[" => {
                self.bump();
                if self.is_op("]") {
                    self.bump();
                    return Ok(Expr::List(vec![]));
                }
                let first = self.parse_expr()?;
                if self.is_kw("for") {
                    // List comprehension: [elt for vars in iter if cond]
                    self.bump();
                    let vars = self.parse_for_vars()?;
                    self.eat_kw("in")?;
                    let iter = self.parse_or()?;
                    let cond = if self.is_kw("if") {
                        self.bump();
                        Some(Box::new(self.parse_or()?))
                    } else { None };
                    self.eat_op("]")?;
                    Ok(Expr::ListComp {
                        elt: Box::new(first), vars, iter: Box::new(iter), cond,
                    })
                } else {
                    let mut elts = vec![first];
                    while self.is_op(",") {
                        self.bump();
                        if self.is_op("]") { break; }
                        elts.push(self.parse_expr()?);
                    }
                    self.eat_op("]")?;
                    Ok(Expr::List(elts))
                }
            }

            // Dict or set literal / comprehension
            Tok::Op(o) if o == "{" => self.parse_dict_or_set(),

            // Identifier: function call or name
            Tok::Ident(w) if !is_keyword(&w) => {
                self.bump();
                if self.is_op("(") {
                    self.bump();
                    let args = self.parse_args()?;
                    self.eat_op(")")?;
                    Ok(Expr::Call { func: w, args })
                } else {
                    Ok(Expr::Name(w))
                }
            }

            other => self.err(format!("unexpected token in expression: {:?}", other)),
        }
    }

    fn parse_dict_or_set(&mut self) -> PResult<Expr> {
        self.eat_op("{")?;
        // Empty braces = empty dict
        if self.is_op("}") {
            self.bump();
            return Ok(Expr::DictLiteral(vec![]));
        }
        let first = self.parse_expr()?;

        if self.is_op(":") {
            // Dict literal or dict comprehension
            self.bump();
            let first_val = self.parse_expr()?;
            if self.is_kw("for") {
                self.bump();
                let vars = self.parse_for_vars()?;
                self.eat_kw("in")?;
                let iter = self.parse_or()?;
                let cond = if self.is_kw("if") {
                    self.bump(); Some(Box::new(self.parse_or()?))
                } else { None };
                self.eat_op("}")?;
                return Ok(Expr::DictComp {
                    key: Box::new(first), val: Box::new(first_val),
                    vars, iter: Box::new(iter), cond,
                });
            }
            let mut pairs = vec![(first, first_val)];
            while self.is_op(",") {
                self.bump();
                if self.is_op("}") { break; }
                let k = self.parse_expr()?;
                self.eat_op(":")?;
                let v = self.parse_expr()?;
                pairs.push((k, v));
            }
            self.eat_op("}")?;
            Ok(Expr::DictLiteral(pairs))
        } else if self.is_kw("for") {
            // Set comprehension
            self.bump();
            let vars = self.parse_for_vars()?;
            self.eat_kw("in")?;
            let iter = self.parse_or()?;
            let cond = if self.is_kw("if") {
                self.bump(); Some(Box::new(self.parse_or()?))
            } else { None };
            self.eat_op("}")?;
            Ok(Expr::SetComp { elt: Box::new(first), vars, iter: Box::new(iter), cond })
        } else {
            // Set literal
            let mut elts = vec![first];
            while self.is_op(",") {
                self.bump();
                if self.is_op("}") { break; }
                elts.push(self.parse_expr()?);
            }
            self.eat_op("}")?;
            Ok(Expr::SetLiteral(elts))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    fn parse(src: &str) -> Program {
        let toks = lex(src).unwrap();
        let mut p = Parser::new(toks);
        p.parse_program().unwrap()
    }

    #[test]
    fn test_augmented_assign() {
        let prog = parse("func main():\n    x = 0\n    x += 1\n");
        assert!(matches!(
            &prog.funcs[0].body[1].kind,
            StmtKind::AugAssign { op: BinOp::Add, .. }
        ));
    }

    #[test]
    fn test_break_continue() {
        let prog = parse("func main():\n    while True:\n        break\n");
        let body = match &prog.funcs[0].body[0].kind {
            StmtKind::While { body, .. } => body,
            _ => panic!(),
        };
        assert!(matches!(body[0].kind, StmtKind::Break));
    }

    #[test]
    fn test_for_iter() {
        let prog = parse("func main():\n    for x in arr:\n        pass\n");
        assert!(matches!(
            &prog.funcs[0].body[0].kind,
            StmtKind::For { iter: Expr::Name(n), .. } if n == "arr"
        ));
    }

    #[test]
    fn test_bitwise() {
        let prog = parse("func main():\n    x = a & b\n");
        assert!(matches!(
            &prog.funcs[0].body[0].kind,
            StmtKind::Assign { value: Expr::Bin { op: BinOp::BitAnd, .. }, .. }
        ));
    }

    #[test]
    fn test_power() {
        let prog = parse("func main():\n    x = 2 ** 10\n");
        assert!(matches!(
            &prog.funcs[0].body[0].kind,
            StmtKind::Assign { value: Expr::Bin { op: BinOp::Pow, .. }, .. }
        ));
    }

    #[test]
    fn test_float_literal() {
        let prog = parse("func main():\n    x = 3.14\n");
        assert!(matches!(
            &prog.funcs[0].body[0].kind,
            StmtKind::Assign { value: Expr::Float(_), .. }
        ));
    }

    #[test]
    fn test_in_operator() {
        let prog = parse("func main():\n    x = a in b\n");
        assert!(matches!(
            &prog.funcs[0].body[0].kind,
            StmtKind::Assign { value: Expr::Bin { op: BinOp::In, .. }, .. }
        ));
    }

    #[test]
    fn test_list_literal() {
        let prog = parse("func main():\n    x = [1, 2, 3]\n");
        assert!(matches!(
            &prog.funcs[0].body[0].kind,
            StmtKind::Assign { value: Expr::List(_), .. }
        ));
    }

    #[test]
    fn test_dict_literal() {
        let prog = parse("func main():\n    x = {'a': 1}\n");
        assert!(matches!(
            &prog.funcs[0].body[0].kind,
            StmtKind::Assign { value: Expr::DictLiteral(_), .. }
        ));
    }

    #[test]
    fn test_generics() {
        let prog = parse("func identity[T](x: T) -> T:\n    return x\n");
        assert_eq!(prog.funcs[0].type_params, vec!["T"]);
    }

    #[test]
    fn test_ref_param() {
        let prog = parse("func fill(arr: ref list[int]) -> void:\n    pass\n");
        assert!(prog.funcs[0].params[0].1); // is_ref == true
    }

    #[test]
    fn test_multi_assign() {
        let prog = parse("func main():\n    a, b = b, a\n");
        assert!(matches!(
            &prog.funcs[0].body[0].kind,
            StmtKind::Assign { targets, .. } if targets.len() == 2
        ));
    }
}
