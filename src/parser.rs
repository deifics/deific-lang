//! Recursive-descent parser with a small Pratt expression layer.

use crate::ast::*;
use crate::lexer::{is_keyword, Tok};

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
        Err(ParseError {
            line: self.line(),
            msg: msg.into(),
        })
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
            Tok::Ident(w) if !is_keyword(&w) => {
                self.bump();
                Ok(w)
            }
            other => self.err(format!("expected identifier, found {:?}", other)),
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek(), Tok::Newline) {
            self.bump();
        }
    }

    pub fn parse_program(&mut self) -> PResult<Program> {
        let mut funcs = Vec::new();
        self.skip_newlines();
        while !matches!(self.peek(), Tok::Eof) {
            if self.is_kw("def") {
                funcs.push(self.parse_func()?);
            } else {
                return self.err("only function definitions are allowed at top level");
            }
            self.skip_newlines();
        }
        Ok(Program { funcs })
    }

    fn parse_func(&mut self) -> PResult<Func> {
        self.eat_kw("def")?;
        let name = self.ident()?;
        self.eat_op("(")?;
        let mut params = Vec::new();
        if !self.is_op(")") {
            loop {
                let pname = self.ident()?;
                self.eat_op(":")?;
                let ty = self.parse_type()?;
                params.push((pname, ty));
                if self.is_op(",") {
                    self.bump();
                } else {
                    break;
                }
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
        Ok(Func {
            name,
            params,
            ret,
            body,
        })
    }

    fn parse_type(&mut self) -> PResult<Type> {
        let name = self.ident()?;
        match name.as_str() {
            "int" => Ok(Type::Int),
            "float" => Ok(Type::Float),
            "bool" => Ok(Type::Bool),
            "str" => Ok(Type::Str),
            "list" => {
                self.eat_op("[")?;
                let inner = self.parse_type()?;
                self.eat_op("]")?;
                Ok(Type::List(Box::new(inner)))
            }
            other => self.err(format!("unknown type '{}'", other)),
        }
    }

    /// Expects NEWLINE INDENT <stmts> DEDENT.
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
            if matches!(self.peek(), Tok::Dedent | Tok::Eof) {
                break;
            }
            stmts.push(self.parse_stmt()?);
        }
        if matches!(self.peek(), Tok::Dedent) {
            self.bump();
        }
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> PResult<Stmt> {
        if self.is_kw("return") {
            self.bump();
            if matches!(self.peek(), Tok::Newline) {
                self.bump();
                return Ok(Stmt::Return(None));
            }
            let e = self.parse_expr()?;
            self.expect_newline()?;
            return Ok(Stmt::Return(Some(e)));
        }
        if self.is_kw("for") {
            return self.parse_for();
        }
        if self.is_kw("while") {
            self.bump();
            let cond = self.parse_expr()?;
            self.eat_op(":")?;
            let body = self.parse_block()?;
            return Ok(Stmt::While { cond, body });
        }
        if self.is_kw("if") {
            return self.parse_if();
        }

        // assignment or bare expression
        let first = self.parse_expr()?;
        if self.is_op(":") {
            self.bump();
            let ty = self.parse_type()?;
            self.eat_op("=")?;
            let value = self.parse_expr()?;
            self.expect_newline()?;
            return Ok(Stmt::Assign {
                target: first,
                ty: Some(ty),
                value,
            });
        }
        if self.is_op("=") {
            self.bump();
            let value = self.parse_expr()?;
            self.expect_newline()?;
            return Ok(Stmt::Assign {
                target: first,
                ty: None,
                value,
            });
        }
        self.expect_newline()?;
        Ok(Stmt::ExprStmt(first))
    }

    fn parse_for(&mut self) -> PResult<Stmt> {
        self.eat_kw("for")?;
        let var = self.ident()?;
        self.eat_kw("in")?;
        // v0 supports only `range(count)`.
        if !self.is_kw("range") {
            return self.err("for-loops currently support only `range(...)`");
        }
        self.bump();
        self.eat_op("(")?;
        let count = self.parse_expr()?;
        self.eat_op(")")?;
        self.eat_op(":")?;
        let body = self.parse_block()?;
        Ok(Stmt::For { var, count, body })
    }

    fn parse_if(&mut self) -> PResult<Stmt> {
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
        Ok(Stmt::If {
            cond,
            then,
            elifs,
            els,
        })
    }

    fn expect_newline(&mut self) -> PResult<()> {
        match self.peek() {
            Tok::Newline => {
                self.bump();
                Ok(())
            }
            Tok::Eof | Tok::Dedent => Ok(()),
            other => self.err(format!("expected end of line, found {:?}", other)),
        }
    }

    // ---- expressions (precedence climbing) ----

    fn parse_expr(&mut self) -> PResult<Expr> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> PResult<Expr> {
        let mut l = self.parse_and()?;
        while self.is_kw("or") {
            self.bump();
            let r = self.parse_and()?;
            l = Expr::Bin {
                op: BinOp::Or,
                l: Box::new(l),
                r: Box::new(r),
            };
        }
        Ok(l)
    }

    fn parse_and(&mut self) -> PResult<Expr> {
        let mut l = self.parse_not()?;
        while self.is_kw("and") {
            self.bump();
            let r = self.parse_not()?;
            l = Expr::Bin {
                op: BinOp::And,
                l: Box::new(l),
                r: Box::new(r),
            };
        }
        Ok(l)
    }

    fn parse_not(&mut self) -> PResult<Expr> {
        if self.is_kw("not") {
            self.bump();
            let e = self.parse_not()?;
            return Ok(Expr::Unary {
                op: UnOp::Not,
                e: Box::new(e),
            });
        }
        self.parse_cmp()
    }

    fn parse_cmp(&mut self) -> PResult<Expr> {
        let l = self.parse_add()?;
        let op = match self.peek() {
            Tok::Op(o) => match o.as_str() {
                "==" => Some(BinOp::Eq),
                "!=" => Some(BinOp::Ne),
                "<" => Some(BinOp::Lt),
                "<=" => Some(BinOp::Le),
                ">" => Some(BinOp::Gt),
                ">=" => Some(BinOp::Ge),
                _ => None,
            },
            _ => None,
        };
        if let Some(op) = op {
            self.bump();
            let r = self.parse_add()?;
            return Ok(Expr::Bin {
                op,
                l: Box::new(l),
                r: Box::new(r),
            });
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
            l = Expr::Bin {
                op,
                l: Box::new(l),
                r: Box::new(r),
            };
        }
        Ok(l)
    }

    fn parse_mul(&mut self) -> PResult<Expr> {
        let mut l = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Tok::Op(o) if o == "*" => BinOp::Mul,
                Tok::Op(o) if o == "/" => BinOp::Div,
                Tok::Op(o) if o == "//" => BinOp::FloorDiv,
                Tok::Op(o) if o == "%" => BinOp::Mod,
                _ => break,
            };
            self.bump();
            let r = self.parse_unary()?;
            l = Expr::Bin {
                op,
                l: Box::new(l),
                r: Box::new(r),
            };
        }
        Ok(l)
    }

    fn parse_unary(&mut self) -> PResult<Expr> {
        if self.is_op("-") {
            self.bump();
            let e = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnOp::Neg,
                e: Box::new(e),
            });
        }
        if self.is_op("*") {
            self.bump();
            let e = self.parse_unary()?;
            return Ok(Expr::Star(Box::new(e)));
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> PResult<Expr> {
        let mut e = self.parse_primary()?;
        loop {
            if self.is_op("[") {
                self.bump();
                let idx = self.parse_expr()?;
                self.eat_op("]")?;
                e = Expr::Index {
                    base: Box::new(e),
                    idx: Box::new(idx),
                };
            } else if self.is_op(".") {
                self.bump();
                let name = self.ident()?;
                self.eat_op("(")?;
                let args = self.parse_args()?;
                self.eat_op(")")?;
                e = Expr::Method {
                    recv: Box::new(e),
                    name,
                    args,
                };
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn parse_args(&mut self) -> PResult<Vec<Expr>> {
        let mut args = Vec::new();
        if self.is_op(")") {
            return Ok(args);
        }
        loop {
            args.push(self.parse_expr()?);
            if self.is_op(",") {
                self.bump();
            } else {
                break;
            }
        }
        Ok(args)
    }

    fn parse_primary(&mut self) -> PResult<Expr> {
        match self.peek().clone() {
            Tok::Int(v) => {
                self.bump();
                Ok(Expr::Int(v))
            }
            Tok::Str(s) => {
                self.bump();
                Ok(Expr::Str(s))
            }
            Tok::Op(o) if o == "(" => {
                self.bump();
                let e = self.parse_expr()?;
                self.eat_op(")")?;
                Ok(e)
            }
            Tok::Ident(w) if w == "True" => {
                self.bump();
                Ok(Expr::Bool(true))
            }
            Tok::Ident(w) if w == "False" => {
                self.bump();
                Ok(Expr::Bool(false))
            }
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
}
