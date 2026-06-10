//! C++ emitter.
//!
//! Strategy: locals are emitted as `auto`, so we lean on the C++ compiler for
//! local type inference (decision Q2). The only types we must spell out are
//! function signatures, which Deific requires the user to annotate. Integer
//! literals get an `LL` suffix so `int` is 64-bit everywhere (decision Q3).

use crate::ast::*;
use std::collections::HashSet;

/// Inlined runtime: a single buffered char reader + buffered writer so the
/// generated program has competitive-programming-grade I/O with no external
/// header. Kept self-contained so the emitted `.cpp` is directly submittable.
pub const RUNTIME: &str = r####"#include <cstdio>
#include <string>
#include <vector>
#include <algorithm>
#include <cstdint>

namespace deific {

// ---- buffered input -------------------------------------------------------
struct FastIn {
    static const int BUF = 1 << 16;
    char buf[BUF];
    int len = 0, pos = 0;
    int gc() {
        if (pos == len) {
            len = (int)std::fread(buf, 1, BUF, stdin);
            pos = 0;
            if (len == 0) return -1;
        }
        return buf[pos++];
    }
};
inline FastIn& in() { static FastIn s; return s; }

inline long long read_int() {
    int c = in().gc();
    while (c != '-' && (c < '0' || c > '9')) {
        if (c == -1) return 0;
        c = in().gc();
    }
    bool neg = false;
    if (c == '-') { neg = true; c = in().gc(); }
    long long x = 0;
    while (c >= '0' && c <= '9') { x = x * 10 + (c - '0'); c = in().gc(); }
    return neg ? -x : x;
}

inline std::string input() {
    std::string s;
    int c = in().gc();
    while (c != '\n' && c != -1) {
        if (c != '\r') s.push_back((char)c);
        c = in().gc();
    }
    return s;
}

inline std::vector<long long> read_ints(long long n) {
    std::vector<long long> v;
    v.reserve(n);
    for (long long i = 0; i < n; i++) v.push_back(read_int());
    return v;
}

inline long long to_int(const std::string& s) {
    long long x = 0, i = 0;
    bool neg = false;
    if (i < (long long)s.size() && (s[i] == '-' || s[i] == '+')) { neg = s[i] == '-'; i++; }
    for (; i < (long long)s.size(); i++) x = x * 10 + (s[i] - '0');
    return neg ? -x : x;
}

// ---- buffered output ------------------------------------------------------
struct FastOut {
    std::string buf;
    FastOut() { buf.reserve(1 << 16); }
    ~FastOut() { std::fwrite(buf.data(), 1, buf.size(), stdout); }
};
inline FastOut& out() { static FastOut s; return s; }

inline void emit(long long x) { out().buf += std::to_string(x); }
inline void emit(int x)       { out().buf += std::to_string((long long)x); }
inline void emit(double x)    { out().buf += std::to_string(x); }
inline void emit(bool x)      { out().buf += (x ? "True" : "False"); }
inline void emit(const std::string& x) { out().buf += x; }
inline void emit(const char* x) { out().buf += x; }

template <class T>
inline void emit(const std::vector<T>& v) {
    for (size_t i = 0; i < v.size(); i++) {
        if (i) out().buf.push_back(' ');
        emit(v[i]);
    }
}

// print(...) : space-separated args, trailing newline (Python default).
inline void print() { out().buf.push_back('\n'); }
template <class T, class... Rest>
inline void print(const T& first, const Rest&... rest) {
    emit(first);
    if constexpr (sizeof...(rest) > 0) out().buf.push_back(' ');
    print(rest...);
}

template <class T> inline long long len(const std::vector<T>& v) { return (long long)v.size(); }
inline long long len(const std::string& s) { return (long long)s.size(); }

} // namespace deific
"####;

pub fn emit_program(p: &Program) -> String {
    let mut s = String::new();
    s.push_str(RUNTIME);
    s.push('\n');
    for f in &p.funcs {
        emit_func(&mut s, f);
        s.push('\n');
    }
    s
}

fn emit_func(s: &mut String, f: &Func) {
    let is_main = f.name == "main" && f.params.is_empty() && f.ret == Type::Void;
    let mut scope: HashSet<String> = HashSet::new();

    if is_main {
        s.push_str("int main() {\n");
    } else {
        let params: Vec<String> = f
            .params
            .iter()
            .map(|(n, t)| {
                scope.insert(n.clone());
                format!("{} {}", t.cpp(), n)
            })
            .collect();
        s.push_str(&format!(
            "{} {}({}) {{\n",
            f.ret.cpp(),
            f.name,
            params.join(", ")
        ));
    }

    for st in &f.body {
        emit_stmt(s, st, &mut scope, 1);
    }

    if is_main {
        s.push_str("    return 0;\n");
    }
    s.push_str("}\n");
}

fn indent(s: &mut String, depth: usize) {
    for _ in 0..depth {
        s.push_str("    ");
    }
}

fn emit_stmt(s: &mut String, st: &Stmt, scope: &mut HashSet<String>, depth: usize) {
    match st {
        Stmt::Assign { target, ty, value } => {
            indent(s, depth);
            match target {
                Expr::Name(name) => {
                    if let Some(t) = ty {
                        scope.insert(name.clone());
                        s.push_str(&format!("{} {} = {};\n", t.cpp(), name, expr(value)));
                    } else if scope.contains(name) {
                        s.push_str(&format!("{} = {};\n", name, expr(value)));
                    } else {
                        scope.insert(name.clone());
                        s.push_str(&format!("auto {} = {};\n", name, expr(value)));
                    }
                }
                other => {
                    // index assignment, e.g. a[i] = x
                    s.push_str(&format!("{} = {};\n", expr(other), expr(value)));
                }
            }
        }
        Stmt::ExprStmt(e) => {
            indent(s, depth);
            s.push_str(&format!("{};\n", expr(e)));
        }
        Stmt::Return(e) => {
            indent(s, depth);
            match e {
                Some(e) => s.push_str(&format!("return {};\n", expr(e))),
                None => s.push_str("return;\n"),
            }
        }
        Stmt::For { var, count, body } => {
            indent(s, depth);
            s.push_str(&format!(
                "for (long long {v} = 0; {v} < ({c}); {v}++) {{\n",
                v = var,
                c = expr(count)
            ));
            let mut inner = scope.clone();
            inner.insert(var.clone());
            for st in body {
                emit_stmt(s, st, &mut inner, depth + 1);
            }
            indent(s, depth);
            s.push_str("}\n");
        }
        Stmt::While { cond, body } => {
            indent(s, depth);
            s.push_str(&format!("while ({}) {{\n", expr(cond)));
            let mut inner = scope.clone();
            for st in body {
                emit_stmt(s, st, &mut inner, depth + 1);
            }
            indent(s, depth);
            s.push_str("}\n");
        }
        Stmt::If {
            cond,
            then,
            elifs,
            els,
        } => {
            indent(s, depth);
            s.push_str(&format!("if ({}) {{\n", expr(cond)));
            let mut inner = scope.clone();
            for st in then {
                emit_stmt(s, st, &mut inner, depth + 1);
            }
            indent(s, depth);
            s.push_str("}");
            for (c, b) in elifs {
                s.push_str(&format!(" else if ({}) {{\n", expr(c)));
                let mut bi = scope.clone();
                for st in b {
                    emit_stmt(s, st, &mut bi, depth + 1);
                }
                indent(s, depth);
                s.push_str("}");
            }
            if let Some(b) = els {
                s.push_str(" else {\n");
                let mut bi = scope.clone();
                for st in b {
                    emit_stmt(s, st, &mut bi, depth + 1);
                }
                indent(s, depth);
                s.push_str("}");
            }
            s.push('\n');
        }
    }
}

fn expr(e: &Expr) -> String {
    match e {
        Expr::Int(v) => format!("{}LL", v),
        Expr::Str(s) => format!("std::string(\"{}\")", s),
        Expr::Bool(b) => b.to_string(),
        Expr::Name(n) => n.clone(),
        Expr::Bin { op, l, r } => {
            let ls = expr(l);
            let rs = expr(r);
            match op {
                BinOp::FloorDiv => format!("(({}) / ({}))", ls, rs), // integer / in C++
                BinOp::And => format!("({} && {})", ls, rs),
                BinOp::Or => format!("({} || {})", ls, rs),
                _ => format!("({} {} {})", ls, binop(*op), rs),
            }
        }
        Expr::Unary { op, e } => match op {
            UnOp::Neg => format!("(-{})", expr(e)),
            UnOp::Not => format!("(!{})", expr(e)),
        },
        Expr::Index { base, idx } => format!("{}[{}]", expr(base), expr(idx)),
        Expr::Star(inner) => expr(inner), // print expands vectors itself
        Expr::Method { recv, name, args } => emit_method(recv, name, args),
        Expr::Call { func, args } => emit_call(func, args),
    }
}

fn emit_call(func: &str, args: &[Expr]) -> String {
    let a: Vec<String> = args.iter().map(expr).collect();
    match func {
        // built-ins mapped into the deific:: runtime
        "print" => format!("deific::print({})", a.join(", ")),
        "input" => "deific::input()".to_string(),
        "read_ints" => format!("deific::read_ints({})", a.join(", ")),
        "read_int" => "deific::read_int()".to_string(),
        "len" => format!("deific::len({})", a.join(", ")),
        "int" => format!("deific::to_int({})", a.join(", ")),
        // user functions pass through
        _ => format!("{}({})", func, a.join(", ")),
    }
}

fn emit_method(recv: &Expr, name: &str, args: &[Expr]) -> String {
    let r = expr(recv);
    let a: Vec<String> = args.iter().map(expr).collect();
    match name {
        "sort" => format!("std::sort({r}.begin(), {r}.end())", r = r),
        "reverse" => format!("std::reverse({r}.begin(), {r}.end())", r = r),
        "append" => format!("{}.push_back({})", r, a.join(", ")),
        "pop" => format!("{}.pop_back()", r),
        _ => format!("{}.{}({})", r, name, a.join(", ")),
    }
}

fn binop(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::FloorDiv | BinOp::And | BinOp::Or => unreachable!(),
    }
}
