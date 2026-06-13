//! C++ emitter.
//!
//! Locals use `auto` so C++ infers types; function signatures spell out types explicitly.
//! Integer literals carry `LL` so `int` is 64-bit everywhere (see README).
//! Source line numbers are emitted as `#line` directives so g++ errors point to .df source.

use crate::ast::*;
use std::collections::HashSet;
use std::cell::RefCell;

thread_local! {
    static STRUCT_NAMES: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

pub const RUNTIME: &str = r####"#include <cstdio>
#include <string>
#include <vector>
#include <algorithm>
#include <cstdint>
#include <unordered_map>
#include <unordered_set>
#include <tuple>
#include <cmath>
#include <functional>
#include <cctype>
#include <climits>
#include <stdexcept>

namespace deific {

// ---- bigint ---------------------------------------------------------------
#ifdef __SIZEOF_INT128__
typedef __int128 bigint;
#else
typedef long long bigint;
#endif

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
inline FastIn& in_buf() { static FastIn s; return s; }

inline long long read_int() {
    int c = in_buf().gc();
    while (c != '-' && (c < '0' || c > '9')) {
        if (c == -1) return 0;
        c = in_buf().gc();
    }
    bool neg = false;
    if (c == '-') { neg = true; c = in_buf().gc(); }
    long long x = 0;
    while (c >= '0' && c <= '9') { x = x * 10 + (c - '0'); c = in_buf().gc(); }
    return neg ? -x : x;
}

inline double read_float() {
    std::string s;
    int c = in_buf().gc();
    while (c == ' ' || c == '\n' || c == '\r' || c == '\t') c = in_buf().gc();
    while (c != ' ' && c != '\n' && c != '\r' && c != '\t' && c != -1) {
        s.push_back((char)c); c = in_buf().gc();
    }
    return s.empty() ? 0.0 : std::stod(s);
}

inline std::string input() {
    std::string s;
    int c = in_buf().gc();
    while (c != '\n' && c != -1) {
        if (c != '\r') s.push_back((char)c);
        c = in_buf().gc();
    }
    return s;
}

inline std::vector<long long> read_ints(long long n) {
    std::vector<long long> v; v.reserve(n);
    for (long long i = 0; i < n; i++) v.push_back(read_int());
    return v;
}
inline std::vector<double> read_floats(long long n) {
    std::vector<double> v; v.reserve(n);
    for (long long i = 0; i < n; i++) v.push_back(read_float());
    return v;
}

inline long long to_int(const std::string& s) {
    if (s.empty()) return 0;
    long long x = 0; size_t i = 0; bool neg = false;
    if (i < s.size() && (s[i] == '-' || s[i] == '+')) { neg = s[i] == '-'; i++; }
    for (; i < s.size(); i++) x = x * 10 + (s[i] - '0');
    return neg ? -x : x;
}
inline long long to_int(long long x) { return x; }
inline long long to_int(double x)    { return (long long)x; }
inline double to_float(const std::string& s) { return std::stod(s); }
inline double to_float(long long x)  { return (double)x; }
inline double to_float(double x)     { return x; }

// ---- buffered output ------------------------------------------------------
struct FastOut {
    std::string buf;
    FastOut() { buf.reserve(1 << 16); }
    ~FastOut() { std::fwrite(buf.data(), 1, buf.size(), stdout); }
};
inline FastOut& out_buf() { static FastOut s; return s; }

inline void emit_val(long long x) { out_buf().buf += std::to_string(x); }
inline void emit_val(int x)       { out_buf().buf += std::to_string((long long)x); }
inline void emit_val(double x)    {
    char tmp[32]; std::snprintf(tmp, sizeof(tmp), "%g", x); out_buf().buf += tmp;
}
inline void emit_val(bool x)      { out_buf().buf += x ? "True" : "False"; }
inline void emit_val(const std::string& x) { out_buf().buf += x; }
inline void emit_val(const char* x)        { out_buf().buf += x; }
#ifdef __SIZEOF_INT128__
inline void emit_val(__int128 x) {
    if (x < 0) { out_buf().buf.push_back('-'); x = -x; }
    if (x == 0) { out_buf().buf.push_back('0'); return; }
    std::string s;
    while (x > 0) { s.push_back('0' + (int)(x % 10)); x /= 10; }
    std::reverse(s.begin(), s.end());
    out_buf().buf += s;
}
#endif
template<class T>
inline void emit_val(const std::vector<T>& v) {
    for (size_t i = 0; i < v.size(); i++) {
        if (i) out_buf().buf.push_back(' ');
        emit_val(v[i]);
    }
}
template<class A, class B>
inline void emit_val(const std::pair<A,B>& p) {
    emit_val(p.first); out_buf().buf.push_back(' '); emit_val(p.second);
}

inline void print() { out_buf().buf.push_back('\n'); }
template<class T, class... Rest>
inline void print(const T& first, const Rest&... rest) {
    emit_val(first);
    if constexpr (sizeof...(rest) > 0) out_buf().buf.push_back(' ');
    print(rest...);
}

// ---- len ------------------------------------------------------------------
template<class T> inline long long len(const std::vector<T>& v) { return (long long)v.size(); }
inline long long len(const std::string& s) { return (long long)s.size(); }
template<class K,class V> inline long long len(const std::unordered_map<K,V>& m) { return (long long)m.size(); }
template<class T> inline long long len(const std::unordered_set<T>& s)            { return (long long)s.size(); }

// ---- str conversion -------------------------------------------------------
inline std::string str(long long x) { return std::to_string(x); }
inline std::string str(double x)    { char t[32]; std::snprintf(t,sizeof(t),"%g",x); return t; }
inline std::string str(bool x)      { return x ? "True" : "False"; }
inline std::string str(const std::string& x) { return x; }
inline std::string str(char x)      { return std::string(1, x); }
inline std::string chr_fn(long long x) { return std::string(1, (char)x); }
inline long long   ord_fn(const std::string& s) { return s.empty() ? 0LL : (long long)(unsigned char)s[0]; }

// ---- math -----------------------------------------------------------------
template<class T> inline T deific_abs(T x) { return x < T{} ? -x : x; }
template<class T> inline T deific_min(T a, T b) { return a < b ? a : b; }
template<class T> inline T deific_max(T a, T b) { return a > b ? a : b; }
template<class T> inline T deific_min(const std::vector<T>& v) { return *std::min_element(v.begin(),v.end()); }
template<class T> inline T deific_max(const std::vector<T>& v) { return *std::max_element(v.begin(),v.end()); }
template<class T> inline T deific_sum(const std::vector<T>& v) { T s=T{}; for(auto& x:v) s+=x; return s; }

inline long long gcd(long long a, long long b) {
    if (a < 0) a = -a; if (b < 0) b = -b;
    while (b) { a %= b; std::swap(a,b); } return a;
}
inline long long lcm(long long a, long long b) { return a/gcd(a,b)*b; }

inline long long pow_int(long long base, long long exp) {
    long long r = 1;
    for (; exp > 0; exp >>= 1) { if (exp & 1) r *= base; base *= base; }
    return r;
}
inline long long pow_mod(long long base, long long exp, long long mod) {
    long long r = 1; base %= mod;
    for (; exp > 0; exp >>= 1) { if (exp & 1) r = r*base%mod; base = base*base%mod; }
    return r;
}

// ---- sorted / reversed ----------------------------------------------------
template<class T> inline std::vector<T> deific_sorted(std::vector<T> v, bool rev=false) {
    if (rev) std::sort(v.begin(),v.end(),std::greater<T>());
    else     std::sort(v.begin(),v.end());
    return v;
}
template<class T> inline std::vector<T> deific_reversed(std::vector<T> v) {
    std::reverse(v.begin(),v.end()); return v;
}

// ---- range (materialises into vector, for comprehensions / generic iter) --
inline std::vector<long long> deific_range(long long stop) {
    std::vector<long long> v; v.reserve(stop>0?stop:0);
    for (long long i=0;i<stop;i++) v.push_back(i);
    return v;
}
inline std::vector<long long> deific_range(long long start, long long stop, long long step=1) {
    std::vector<long long> v;
    if (step>0) for (long long i=start;i<stop;i+=step) v.push_back(i);
    else        for (long long i=start;i>stop;i+=step) v.push_back(i);
    return v;
}

// ---- enumerate / zip ------------------------------------------------------
template<class T>
inline std::vector<std::pair<long long,T>> enumerate(const std::vector<T>& v, long long start=0) {
    std::vector<std::pair<long long,T>> out; out.reserve(v.size());
    for (size_t i=0;i<v.size();i++) out.push_back({(long long)i+start, v[i]});
    return out;
}
template<class A,class B>
inline std::vector<std::pair<A,B>> zip(const std::vector<A>& a, const std::vector<B>& b) {
    size_t n=std::min(a.size(),b.size());
    std::vector<std::pair<A,B>> out; out.reserve(n);
    for (size_t i=0;i<n;i++) out.push_back({a[i],b[i]});
    return out;
}

// ---- in operator ----------------------------------------------------------
template<class T>
inline bool deific_in(const T& v, const std::vector<T>& c) {
    return std::find(c.begin(),c.end(),v)!=c.end();
}
inline bool deific_in(const std::string& v, const std::string& s) { return s.find(v)!=std::string::npos; }
template<class K,class V>
inline bool deific_in(const K& k, const std::unordered_map<K,V>& m) { return m.count(k)>0; }
template<class T>
inline bool deific_in(const T& v, const std::unordered_set<T>& s) { return s.count(v)>0; }

// ---- slice ----------------------------------------------------------------
template<class T>
inline std::vector<T> deific_slice(const std::vector<T>& v, long long a, long long b) {
    long long n=(long long)v.size();
    if(a<0) a=std::max(0LL,n+a); if(b<0) b=std::max(0LL,n+b);
    a=std::min(a,n); b=std::min(b,n);
    if(a>=b) return {};
    return std::vector<T>(v.begin()+a, v.begin()+b);
}
template<class T>
inline std::vector<T> deific_slice(const std::vector<T>& v, long long a, long long b, long long step) {
    long long n=(long long)v.size();
    if(a<0) a=n+a; if(b<0) b=n+b;
    a=std::max(0LL,std::min(a,n)); b=std::max(0LL,std::min(b,n));
    std::vector<T> out;
    if(step>0) for(long long i=a;i<b;i+=step) out.push_back(v[i]);
    else       for(long long i=a;i>b;i+=step) out.push_back(v[i]);
    return out;
}
inline std::string deific_slice(const std::string& s, long long a, long long b) {
    long long n=(long long)s.size();
    if(a<0) a=std::max(0LL,n+a); if(b<0) b=std::max(0LL,n+b);
    a=std::min(a,n); b=std::min(b,n);
    if(a>=b) return "";
    return s.substr(a, b-a);
}

// ---- string methods -------------------------------------------------------
inline std::string str_upper(std::string s) {
    for(char& c:s) c=(char)std::toupper((unsigned char)c); return s;
}
inline std::string str_lower(std::string s) {
    for(char& c:s) c=(char)std::tolower((unsigned char)c); return s;
}
inline std::string str_strip(const std::string& s) {
    size_t a=0, b=s.size();
    while(a<b && std::isspace((unsigned char)s[a])) a++;
    while(b>a && std::isspace((unsigned char)s[b-1])) b--;
    return s.substr(a,b-a);
}
inline std::string str_lstrip(const std::string& s) {
    size_t a=0;
    while(a<s.size() && std::isspace((unsigned char)s[a])) a++;
    return s.substr(a);
}
inline std::string str_rstrip(const std::string& s) {
    size_t b=s.size();
    while(b>0 && std::isspace((unsigned char)s[b-1])) b--;
    return s.substr(0,b);
}
inline std::vector<std::string> str_split(const std::string& s) {
    std::vector<std::string> out; std::string cur;
    for(char c:s) {
        if(std::isspace((unsigned char)c)) { if(!cur.empty()){out.push_back(cur);cur.clear();} }
        else cur.push_back(c);
    }
    if(!cur.empty()) out.push_back(cur);
    return out;
}
inline std::vector<std::string> str_split(const std::string& s, const std::string& sep) {
    std::vector<std::string> out; size_t pos=0, found;
    while((found=s.find(sep,pos))!=std::string::npos) {
        out.push_back(s.substr(pos,found-pos)); pos=found+sep.size();
    }
    out.push_back(s.substr(pos)); return out;
}
template<class T>
inline std::string str_join(const std::string& sep, const std::vector<T>& v) {
    std::string out;
    for(size_t i=0;i<v.size();i++) { if(i) out+=sep; out+=str(v[i]); }
    return out;
}
inline bool str_startswith(const std::string& s, const std::string& p) {
    return s.size()>=p.size() && s.compare(0,p.size(),p)==0;
}
inline bool str_endswith(const std::string& s, const std::string& p) {
    return s.size()>=p.size() && s.compare(s.size()-p.size(),p.size(),p)==0;
}
inline long long str_find(const std::string& s, const std::string& sub, long long start=0) {
    auto pos=s.find(sub,(size_t)start);
    return pos==std::string::npos ? -1LL : (long long)pos;
}
inline std::string str_replace(std::string s, const std::string& old_s, const std::string& new_s) {
    size_t pos=0;
    while((pos=s.find(old_s,pos))!=std::string::npos) { s.replace(pos,old_s.size(),new_s); pos+=new_s.size(); }
    return s;
}
inline long long str_count(const std::string& s, const std::string& sub) {
    long long n=0; size_t pos=0;
    while((pos=s.find(sub,pos))!=std::string::npos) { n++; pos+=sub.size(); } return n;
}
inline std::string str_zfill(const std::string& s, long long w) {
    return (long long)s.size()>=w ? s : std::string(w-s.size(),'0')+s;
}

// ---- panic / assert -------------------------------------------------------
#ifdef DEIFIC_TEST
inline void panic(const std::string& msg) { throw std::runtime_error(msg); }
#else
inline void panic(const std::string& msg) {
    std::fprintf(stderr, "panic: %s\n", msg.c_str());
    std::exit(1);
}
#endif
inline void deific_assert(bool cond, const char* msg) {
    if (!cond) panic(std::string("assertion failed: ") + msg);
}

// ---- defer (RAII scope guard) ---------------------------------------------
struct Defer {
    std::function<void()> fn;
    ~Defer() { fn(); }
};

} // namespace deific
"####;

// ---- unique name generator ------------------------------------------------

static GENSYM: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn gensym(prefix: &str) -> String {
    let n = GENSYM.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("_df_{}_{}", prefix, n)
}

// ---- program entry --------------------------------------------------------

pub fn emit_program(p: &Program, src_path: &str) -> String {
    emit_program_inner(p, src_path, false)
}

pub fn emit_test_program(p: &Program, src_path: &str) -> (String, Vec<String>) {
    let test_fns: Vec<String> = p.funcs.iter()
        .filter(|f| f.name.starts_with("test_") && f.params.is_empty() && f.ret == Type::Void)
        .map(|f| f.name.clone())
        .collect();
    let cpp = emit_program_inner(p, src_path, true);
    (cpp, test_fns)
}

fn emit_program_inner(p: &Program, src_path: &str, test_mode: bool) -> String {
    // Register struct names for use in expr()
    STRUCT_NAMES.with(|sn| {
        let mut set = sn.borrow_mut();
        set.clear();
        for s in &p.structs { set.insert(s.name.clone()); }
    });

    let mut s = String::new();
    if test_mode { s.push_str("#define DEIFIC_TEST\n"); }
    s.push_str(RUNTIME);
    s.push('\n');

    // Struct definitions
    for st in &p.structs {
        s.push_str(&format!("struct {} {{\n", st.name));
        for (fname, ftype) in &st.fields {
            s.push_str(&format!("    {} {};\n", ftype.cpp(), fname));
        }
        s.push_str("};\n");
    }
    if !p.structs.is_empty() { s.push('\n'); }

    // Global variables
    for g in &p.globals {
        if let Some(ty) = &g.ty {
            s.push_str(&format!("{} {} = {};\n", ty.cpp(), g.name, expr(&g.value)));
        } else {
            s.push_str(&format!("auto {} = {};\n", g.name, expr(&g.value)));
        }
    }
    if !p.globals.is_empty() { s.push('\n'); }

    for f in &p.funcs {
        emit_func(&mut s, f, src_path, &p.globals);
        s.push('\n');
    }

    if test_mode {
        let test_fns: Vec<&str> = p.funcs.iter()
            .filter(|f| f.name.starts_with("test_") && f.params.is_empty() && f.ret == Type::Void)
            .map(|f| f.name.as_str())
            .collect();
        s.push_str("int main() {\n");
        s.push_str("    int _passed = 0, _failed = 0;\n");
        for name in test_fns {
            s.push_str(&format!(
                "    try {{ {name}(); deific::print(std::string(\"PASS {name}\")); _passed++; }}\n\
                 \x20   catch (const std::exception& _e) {{ deific::print(std::string(\"FAIL {name}: \") + _e.what()); _failed++; }}\n",
                name = name
            ));
        }
        s.push_str("    deific::print(_passed, std::string(\"passed,\"), _failed, std::string(\"failed\"));\n");
        s.push_str("    return _failed > 0 ? 1 : 0;\n}\n");
    }

    s
}

fn emit_func(s: &mut String, f: &Func, src_path: &str, globals: &[crate::ast::GlobalVar]) {
    let is_main = f.name == "main" && f.params.is_empty() && f.ret == Type::Void;
    // Pre-seed scope with any names this function declares `global`
    let declared_globals: HashSet<String> = f.body.iter().filter_map(|st| {
        if let StmtKind::Global(names) = &st.kind { Some(names.iter().cloned()) } else { None }
    }).flatten().collect();
    // Also pre-seed all program-level globals so they're never re-declared with `auto`
    let global_names: HashSet<String> = globals.iter().map(|g| g.name.clone()).collect();
    let mut scope: HashSet<String> = declared_globals.union(&global_names).cloned().collect();

    // Generic template prefix
    if !f.type_params.is_empty() {
        let tps: Vec<String> = f.type_params.iter().map(|t| format!("typename {}", t)).collect();
        s.push_str(&format!("template<{}>\n", tps.join(",")));
    }

    if is_main {
        s.push_str("int main() {\n");
    } else {
        let params: Vec<String> = f.params.iter().map(|(n, is_ref, t)| {
            scope.insert(n.clone());
            if *is_ref {
                format!("{}&{}", t.cpp(), n)
            } else {
                format!("{} {}", t.cpp(), n)
            }
        }).collect();
        s.push_str(&format!("{} {}({}) {{\n", f.ret.cpp(), f.name, params.join(",")));
    }

    for st in &f.body {
        emit_stmt(s, st, &mut scope, 1, src_path);
    }

    if is_main {
        s.push_str("    return 0;\n");
    }
    s.push_str("}\n");
}

fn indent(s: &mut String, depth: usize) {
    for _ in 0..depth { s.push_str("    "); }
}

fn emit_stmt(s: &mut String, st: &Stmt, scope: &mut HashSet<String>, depth: usize, src: &str) {
    // Emit a #line directive so g++ errors point to the .df source
    indent(s, depth);
    s.push_str(&format!("#line {} \"{}\"\n", st.line, src.replace('\\', "/")));

    match &st.kind {
        StmtKind::Global(_) => { /* resolved at function scope setup — no C++ output needed */ }

        StmtKind::Defer(e) => {
            let n = gensym("defer");
            indent(s, depth);
            s.push_str(&format!("deific::Defer {}{{[&](){{ {}; }}}};\n", n, expr(e)));
        }

        StmtKind::Pass => {
            indent(s, depth);
            s.push_str("/* pass */\n");
        }

        StmtKind::Break => {
            indent(s, depth);
            s.push_str("break;\n");
        }

        StmtKind::Continue => {
            indent(s, depth);
            s.push_str("continue;\n");
        }

        StmtKind::AugAssign { target, op, value } => {
            indent(s, depth);
            let op_str = match op {
                BinOp::Add => "+=", BinOp::Sub => "-=", BinOp::Mul => "*=",
                BinOp::Div => "/=", BinOp::FloorDiv => "/=",  // floor div augmented still uses /=
                BinOp::Mod => "%=", BinOp::Pow => "",          // ** handled specially
                BinOp::BitAnd => "&=", BinOp::BitOr => "|=", BinOp::BitXor => "^=",
                BinOp::Shl => "<<=", BinOp::Shr => ">>=",
                _ => unreachable!("invalid aug op"),
            };
            if *op == BinOp::Pow {
                // target = pow_int(target, value)  — emit as reassign
                s.push_str(&format!("{} = deific::pow_int({}, {});\n",
                    expr(target), expr(target), expr(value)));
            } else {
                s.push_str(&format!("{} {} {};\n", expr(target), op_str, expr(value)));
            }
        }

        StmtKind::Assign { targets, ty, value } => {
            if targets.len() > 1 {
                emit_multi_assign(s, targets, value, scope, depth);
            } else {
                let target = &targets[0];
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
                        s.push_str(&format!("{} = {};\n", expr(other), expr(value)));
                    }
                }
            }
        }

        StmtKind::ExprStmt(e) => {
            indent(s, depth);
            s.push_str(&format!("{};\n", expr(e)));
        }

        StmtKind::Return(e) => {
            indent(s, depth);
            match e {
                Some(e) => s.push_str(&format!("return {};\n", expr(e))),
                None    => s.push_str("return;\n"),
            }
        }

        StmtKind::For { vars, iter, body } => {
            emit_for(s, vars, iter, body, scope, depth, src);
        }

        StmtKind::While { cond, body } => {
            indent(s, depth);
            s.push_str(&format!("while ({}) {{\n", expr(cond)));
            let mut inner = scope.clone();
            for st in body { emit_stmt(s, st, &mut inner, depth + 1, src); }
            indent(s, depth);
            s.push_str("}\n");
        }

        StmtKind::If { cond, then, elifs, els } => {
            indent(s, depth);
            s.push_str(&format!("if ({}) {{\n", expr(cond)));
            let mut inner = scope.clone();
            for st in then { emit_stmt(s, st, &mut inner, depth + 1, src); }
            indent(s, depth);
            s.push('}');
            for (c, b) in elifs {
                s.push_str(&format!(" else if ({}) {{\n", expr(c)));
                let mut bi = scope.clone();
                for st in b { emit_stmt(s, st, &mut bi, depth + 1, src); }
                indent(s, depth);
                s.push('}');
            }
            if let Some(b) = els {
                s.push_str(" else {\n");
                let mut bi = scope.clone();
                for st in b { emit_stmt(s, st, &mut bi, depth + 1, src); }
                indent(s, depth);
                s.push('}');
            }
            s.push('\n');
        }
    }
}

// Emit `a, b = rhs` (multi-target assignment)
fn emit_multi_assign(
    s: &mut String,
    targets: &[Expr],
    value: &Expr,
    scope: &mut HashSet<String>,
    depth: usize,
) {
    // Evaluate RHS into temporaries first (prevents aliasing: a, b = b, a)
    let tmp = gensym("t");
    match value {
        Expr::Tuple(elts) if elts.len() == targets.len() => {
            // RHS is an inline tuple — evaluate each element into its own temp
            let tmps: Vec<String> = (0..elts.len()).map(|i| format!("{}{}", tmp, i)).collect();
            for (i, elt) in elts.iter().enumerate() {
                indent(s, depth);
                s.push_str(&format!("auto {} = {};\n", tmps[i], expr(elt)));
            }
            for (i, tgt) in targets.iter().enumerate() {
                indent(s, depth);
                match tgt {
                    Expr::Name(name) => {
                        if scope.contains(name) {
                            s.push_str(&format!("{} = {};\n", name, tmps[i]));
                        } else {
                            scope.insert(name.clone());
                            s.push_str(&format!("auto {} = {};\n", name, tmps[i]));
                        }
                    }
                    other => s.push_str(&format!("{} = {};\n", expr(other), tmps[i])),
                }
            }
        }
        _ => {
            // RHS is a single expression returning a tuple/pair — use structured binding
            indent(s, depth);
            let target_names: Vec<String> = targets.iter().map(|t| match t {
                Expr::Name(n) => { scope.insert(n.clone()); n.clone() }
                other => expr(other),
            }).collect();
            s.push_str(&format!("auto [{}] = {};\n", target_names.join(","), expr(value)));
        }
    }
}

// Emit a for loop, special-casing range/enumerate/zip
fn emit_for(
    s: &mut String,
    vars: &[String],
    iter: &Expr,
    body: &[Stmt],
    scope: &mut HashSet<String>,
    depth: usize,
    src: &str,
) {
    let mut inner = scope.clone();
    for v in vars { inner.insert(v.clone()); }

    indent(s, depth);

    match iter {
        // for x in range(n)  /  range(a,b)  /  range(a,b,step)
        Expr::Call { func, args } if func == "range" && vars.len() == 1 => {
            let v = &vars[0];
            match args.len() {
                1 => s.push_str(&format!(
                    "for (long long {v} = 0LL; {v} < ({stop}); {v}++) {{\n",
                    v = v, stop = expr(&args[0])
                )),
                2 => s.push_str(&format!(
                    "for (long long {v} = ({start}); {v} < ({stop}); {v}++) {{\n",
                    v = v, start = expr(&args[0]), stop = expr(&args[1])
                )),
                3 => s.push_str(&format!(
                    "for (long long {v} = ({start}); {step_pos}; {v} += ({step})) {{\n",
                    v = v, start = expr(&args[0]),
                    step_pos = if matches!(&args[2], Expr::Unary { op: UnOp::Neg, .. }) {
                        format!("{v} > ({})", expr(&args[1]))
                    } else {
                        format!("{v} < ({})", expr(&args[1]))
                    },
                    step = expr(&args[2])
                )),
                _ => s.push_str(&format!("for (auto {} : deific::deific_range({:?})) {{\n", v, args)),
            }
        }

        // for i, x in enumerate(arr)  /  enumerate(arr, start)
        Expr::Call { func, args } if func == "enumerate" && vars.len() == 2 => {
            let ivar = &vars[0];
            let xvar = &vars[1];
            let arr = expr(&args[0]);
            let start = args.get(1).map(|a| expr(a)).unwrap_or("0LL".into());
            s.push_str(&format!(
                "for (long long {i} = ({start}); {i} < ({start}) + deific::len({a}); {i}++) {{\n",
                i = ivar, start = start, a = arr
            ));
            indent(s, depth + 1);
            s.push_str(&format!("auto {} = {}[{} - ({start})];\n",
                xvar, arr, ivar, start = start));
            for st in body { emit_stmt(s, st, &mut inner, depth + 1, src); }
            indent(s, depth);
            s.push_str("}\n");
            return;
        }

        // for x, y in zip(a, b)
        Expr::Call { func, args } if func == "zip" && vars.len() == 2 && args.len() == 2 => {
            let xvar = &vars[0];
            let yvar = &vars[1];
            let a = expr(&args[0]);
            let b = expr(&args[1]);
            let tmp = gensym("zi");
            s.push_str(&format!(
                "for (long long {tmp} = 0; {tmp} < (long long)std::min({a}.size(), {b}.size()); {tmp}++) {{\n",
                tmp = tmp, a = a, b = b
            ));
            indent(s, depth + 1);
            s.push_str(&format!("auto {} = {}[{}];\n", xvar, a, tmp));
            indent(s, depth + 1);
            s.push_str(&format!("auto {} = {}[{}];\n", yvar, b, tmp));
            for st in body { emit_stmt(s, st, &mut inner, depth + 1, src); }
            indent(s, depth);
            s.push_str("}\n");
            return;
        }

        // Generic: for x in iterable  or  for x, y in iterable_of_pairs
        _ => {
            if vars.len() == 1 {
                s.push_str(&format!("for (auto {} : {}) {{\n", vars[0], expr(iter)));
            } else {
                let binding = format!("[{}]", vars.join(","));
                s.push_str(&format!("for (auto {} : {}) {{\n", binding, expr(iter)));
            }
        }
    }

    for st in body { emit_stmt(s, st, &mut inner, depth + 1, src); }
    indent(s, depth);
    s.push_str("}\n");
}

// ---- expression emitter ---------------------------------------------------

fn expr(e: &Expr) -> String {
    match e {
        Expr::Int(v)   => format!("{}LL", v),
        Expr::Float(v) => {
            // Always emit with a decimal point so C++ treats it as double
            let s = format!("{}", v);
            if s.contains('.') || s.contains('e') || s.contains('E') { s }
            else { format!("{}.0", s) }
        }
        Expr::Str(s)  => format!("std::string(\"{}\")", s),
        Expr::Bool(b) => b.to_string(),
        Expr::None    => "0LL".into(),   // None → 0 (no GC/null in Deific)
        Expr::Name(n) => n.clone(),

        Expr::Tuple(elts) => {
            let cs: Vec<String> = elts.iter().map(expr).collect();
            format!("std::make_tuple({})", cs.join(","))
        }

        Expr::List(elts) => {
            let cs: Vec<String> = elts.iter().map(expr).collect();
            if cs.is_empty() {
                // Type unknown; emit as empty init-list wrapped in vector
                "/* empty list — annotate type: x: list[int] = [] */{}".into()
            } else {
                format!("{{{}}}", cs.join(","))
            }
        }

        Expr::ListComp { elt, vars, iter, cond } => {
            emit_comprehension("_lc", elt, vars, iter, cond.as_deref(), "list")
        }
        Expr::DictComp { key, val, vars, iter, cond } => {
            emit_dict_comp(key, val, vars, iter, cond.as_deref())
        }
        Expr::SetComp { elt, vars, iter, cond } => {
            emit_comprehension("_sc", elt, vars, iter, cond.as_deref(), "set")
        }

        Expr::DictLiteral(pairs) => {
            if pairs.is_empty() { return "{}".into(); }
            let ps: Vec<String> = pairs.iter()
                .map(|(k, v)| format!("{{{},{}}}", expr(k), expr(v)))
                .collect();
            format!("{{{}}}", ps.join(","))
        }
        Expr::SetLiteral(elts) => {
            let cs: Vec<String> = elts.iter().map(expr).collect();
            format!("{{{}}}", cs.join(","))
        }

        Expr::Bin { op, l, r } => emit_bin(*op, l, r),
        Expr::Unary { op, e } => match op {
            UnOp::Neg    => format!("(-{})", expr(e)),
            UnOp::Not    => format!("(!{})", expr(e)),
            UnOp::BitNot => format!("(~{})", expr(e)),
        },

        Expr::Index { base, idx } => format!("{}[{}]", expr(base), expr(idx)),
        Expr::Star(inner) => expr(inner),

        Expr::Slice { base, start, stop, step } => {
            let b = expr(base);
            let start_s = start.as_deref().map(expr).unwrap_or("0LL".into());
            let stop_s  = stop.as_deref().map(expr)
                .unwrap_or_else(|| format!("deific::len({})", b));
            match step {
                None => format!("deific::deific_slice({},{},{})", b, start_s, stop_s),
                Some(step) => format!("deific::deific_slice({},{},{},{})", b, start_s, stop_s, expr(step)),
            }
        }

        Expr::Field { recv, name } => format!("{}.{}", expr(recv), name),
        Expr::Method { recv, name, args } => emit_method(recv, name, args),
        Expr::Call    { func, args }      => emit_call(func, args),
    }
}

fn emit_bin(op: BinOp, l: &Expr, r: &Expr) -> String {
    let ls = expr(l);
    let rs = expr(r);
    match op {
        BinOp::FloorDiv => format!("(({})/({}))",&ls,&rs),
        BinOp::And      => format!("({}&&{})", ls, rs),
        BinOp::Or       => format!("({}||{})", ls, rs),
        BinOp::Pow      => format!("deific::pow_int({},{})", ls, rs),
        BinOp::In       => format!("deific::deific_in({},{})", ls, rs),
        BinOp::NotIn    => format!("(!deific::deific_in({},{}))", ls, rs),
        _ => {
            let sym = match op {
                BinOp::Add => "+", BinOp::Sub => "-", BinOp::Mul => "*",
                BinOp::Div => "/", BinOp::Mod => "%",
                BinOp::Eq  => "==", BinOp::Ne => "!=",
                BinOp::Lt  => "<",  BinOp::Le => "<=",
                BinOp::Gt  => ">",  BinOp::Ge => ">=",
                BinOp::BitAnd => "&", BinOp::BitOr => "|", BinOp::BitXor => "^",
                BinOp::Shl => "<<", BinOp::Shr => ">>",
                _ => unreachable!(),
            };
            format!("({}{}{})", ls, sym, rs)
        }
    }
}

fn emit_call(func: &str, args: &[Expr]) -> String {
    let a: Vec<String> = args.iter().map(expr).collect();
    match func {
        "print"       => format!("deific::print({})", a.join(",")),
        "input"       => "deific::input()".into(),
        "read_int"    => "deific::read_int()".into(),
        "read_float"  => "deific::read_float()".into(),
        "read_ints"   => format!("deific::read_ints({})", a.join(",")),
        "read_floats" => format!("deific::read_floats({})", a.join(",")),
        "len"         => format!("deific::len({})", a.join(",")),
        "int"         => format!("deific::to_int({})", a.join(",")),
        "float"       => format!("deific::to_float({})", a.join(",")),
        "str"         => format!("deific::str({})", a.join(",")),
        "chr"         => format!("deific::chr_fn({})", a.join(",")),
        "ord"         => format!("deific::ord_fn({})", a.join(",")),
        "abs"         => format!("deific::deific_abs({})", a.join(",")),
        "min"         => {
            if a.len() == 1 { format!("deific::deific_min({})", a[0]) }
            else            { format!("deific::deific_min({},{})", a[0], a[1]) }
        }
        "max"         => {
            if a.len() == 1 { format!("deific::deific_max({})", a[0]) }
            else            { format!("deific::deific_max({},{})", a[0], a[1]) }
        }
        "sum"         => format!("deific::deific_sum({})", a.join(",")),
        "sorted"      => {
            if a.len() == 1 { format!("deific::deific_sorted({})", a[0]) }
            else            { format!("deific::deific_sorted({},{})", a[0], a[1]) }
        }
        "reversed"    => format!("deific::deific_reversed({})", a.join(",")),
        "range"       => format!("deific::deific_range({})", a.join(",")),
        "enumerate"   => format!("deific::enumerate({})", a.join(",")),
        "zip"         => format!("deific::zip({})", a.join(",")),
        "gcd"         => format!("deific::gcd({})", a.join(",")),
        "lcm"         => format!("deific::lcm({})", a.join(",")),
        "pow_mod"     => format!("deific::pow_mod({})", a.join(",")),
        "panic"       => format!("deific::panic({})", a.join(",")),
        "assert"      => {
            let cond = a.first().map(|s| s.as_str()).unwrap_or("true");
            format!("deific::deific_assert({}, \"{}\")", cond, cond.replace('"', "\\\""))
        }
        _ => {
            // Struct construction: Point(1, 2) → Point{1LL, 2LL}
            let is_struct = STRUCT_NAMES.with(|sn| sn.borrow().contains(func));
            if is_struct {
                format!("{}{{{}}}", func, a.join(","))
            } else {
                format!("{}({})", func, a.join(","))
            }
        }
    }
}

fn emit_method(recv: &Expr, name: &str, args: &[Expr]) -> String {
    let r = expr(recv);
    let a: Vec<String> = args.iter().map(expr).collect();
    match name {
        // list (vector) methods
        "sort"      => format!("std::sort({r}.begin(),{r}.end())", r=r),
        "reverse"   => format!("std::reverse({r}.begin(),{r}.end())", r=r),
        "append"    => format!("{}.push_back({})", r, a.join(",")),
        "pop"       => {
            if a.is_empty() { format!("{}.pop_back()", r) }
            else { format!("{}.erase({r}.begin()+({}))", r, a[0], r=r) }
        }
        "insert"    => format!("{r}.insert({r}.begin()+({idx}),{val})",
                                r=r, idx=a[0], val=a[1]),
        "extend"    => format!("{r}.insert({r}.end(),({src}).begin(),({src}).end())",
                                r=r, src=a[0]),
        "count"     => {
            // works for both strings and vectors
            format!("deific::str_count({},{})", r, a[0])
        }
        "index"     => format!("(long long)(std::find({r}.begin(),{r}.end(),{v})-{r}.begin())",
                                r=r, v=a[0]),
        "clear"     => format!("{}.clear()", r),

        // set methods
        "add"     => format!("{}.insert({})", r, a[0]),
        "discard" => format!("{}.erase({})", r, a[0]),
        "remove"  => format!("{}.erase({})", r, a[0]),

        // dict methods
        "keys"   => format!("/* dict.keys() not directly iterable — use for k in d: */\n{}", r),
        "values" => format!("/* dict.values() not directly iterable */\n{}", r),
        "get"    => {
            if a.len() == 2 {
                format!("({r}.count({k}) ? {r}.at({k}) : ({default}))",
                    r=r, k=a[0], default=a[1])
            } else {
                format!("{r}.count({k}) ? {r}.at({k}) : decltype({r}.begin()->second){{}}",
                    r=r, k=a[0])
            }
        }

        // string methods
        "upper"      => format!("deific::str_upper({})", r),
        "lower"      => format!("deific::str_lower({})", r),
        "strip"      => format!("deific::str_strip({})", r),
        "lstrip"     => format!("deific::str_lstrip({})", r),
        "rstrip"     => format!("deific::str_rstrip({})", r),
        "split"      => {
            if a.is_empty() { format!("deific::str_split({})", r) }
            else            { format!("deific::str_split({},{})", r, a[0]) }
        }
        "join"       => format!("deific::str_join({},{})", r, a[0]),
        "startswith" => format!("deific::str_startswith({},{})", r, a[0]),
        "endswith"   => format!("deific::str_endswith({},{})", r, a[0]),
        "find"       => {
            if a.len() == 1 { format!("deific::str_find({},{})", r, a[0]) }
            else            { format!("deific::str_find({},{},{})", r, a[0], a[1]) }
        }
        "replace"    => format!("deific::str_replace({},{},{})", r, a[0], a[1]),
        "zfill"      => format!("deific::str_zfill({},{})", r, a[0]),

        // sort with key / reverse arg (simplified: only sort/reverse supported)
        _ => format!("{}.{}({})", r, name, a.join(",")),
    }
}

// Emit a list or set comprehension as an immediately-invoked lambda.
// Uses a compute-lambda to infer element type without scoping issues.
fn emit_comprehension(
    prefix: &str,
    elt: &Expr,
    vars: &[String],
    iter: &Expr,
    cond: Option<&Expr>,
    _kind: &str,
) -> String {
    let tmp  = gensym(prefix);
    let cmp  = gensym("cmp");
    let iter_s = expr(iter);
    let elt_s  = expr(elt);
    let cond_part = cond.map(|c| format!("if ({}) ", expr(c))).unwrap_or_default();

    if vars.len() == 1 {
        let var = &vars[0];
        format!(
            "[&](){{ \
auto {cmp} = [&](auto {var}) {{ return {elt}; }}; \
using _ET_{tmp} = std::decay_t<decltype({cmp}(*std::begin({iter})))>; \
std::vector<_ET_{tmp}> {tmp}; \
for (auto {var} : {iter}) {{ {cond}{tmp}.push_back({elt}); }} \
return {tmp}; }}()",
            cmp=cmp, var=var, elt=elt_s, iter=iter_s, tmp=tmp, cond=cond_part
        )
    } else {
        let binding = vars.join(",");
        format!(
            "[&](){{ \
auto {cmp} = [&](auto _p_) {{ auto [{binding}] = _p_; return {elt}; }}; \
using _ET_{tmp} = std::decay_t<decltype({cmp}(*std::begin({iter})))>; \
std::vector<_ET_{tmp}> {tmp}; \
for (auto [{binding}] : {iter}) {{ {cond}{tmp}.push_back({elt}); }} \
return {tmp}; }}()",
            cmp=cmp, binding=binding, elt=elt_s, iter=iter_s, tmp=tmp, cond=cond_part
        )
    }
}

fn emit_dict_comp(
    key: &Expr,
    val: &Expr,
    vars: &[String],
    iter: &Expr,
    cond: Option<&Expr>,
) -> String {
    let tmp  = gensym("dc");
    let kcmp = gensym("kc");
    let vcmp = gensym("vc");
    let iter_s = expr(iter);
    let key_s  = expr(key);
    let val_s  = expr(val);
    let cond_part = cond.map(|c| format!("if ({}) ", expr(c))).unwrap_or_default();

    if vars.len() == 1 {
        let var = &vars[0];
        format!(
            "[&](){{ \
auto {kcmp} = [&](auto {var}) {{ return {key}; }}; \
auto {vcmp} = [&](auto {var}) {{ return {val}; }}; \
using _KT_{tmp} = std::decay_t<decltype({kcmp}(*std::begin({iter})))>; \
using _VT_{tmp} = std::decay_t<decltype({vcmp}(*std::begin({iter})))>; \
std::unordered_map<_KT_{tmp},_VT_{tmp}> {tmp}; \
for (auto {var} : {iter}) {{ {cond}{tmp}[{key}] = {val}; }} \
return {tmp}; }}()",
            kcmp=kcmp, vcmp=vcmp, var=var, key=key_s, val=val_s,
            iter=iter_s, tmp=tmp, cond=cond_part
        )
    } else {
        let binding = vars.join(",");
        format!(
            "[&](){{ \
auto {kcmp} = [&](auto _p_) {{ auto [{binding}] = _p_; return {key}; }}; \
auto {vcmp} = [&](auto _p_) {{ auto [{binding}] = _p_; return {val}; }}; \
using _KT_{tmp} = std::decay_t<decltype({kcmp}(*std::begin({iter})))>; \
using _VT_{tmp} = std::decay_t<decltype({vcmp}(*std::begin({iter})))>; \
std::unordered_map<_KT_{tmp},_VT_{tmp}> {tmp}; \
for (auto [{binding}] : {iter}) {{ {cond}{tmp}[{key}] = {val}; }} \
return {tmp}; }}()",
            kcmp=kcmp, vcmp=vcmp, binding=binding, key=key_s, val=val_s,
            iter=iter_s, tmp=tmp, cond=cond_part
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;
    use crate::parser::Parser;

    fn compile(src: &str) -> String {
        let toks = lex(src).unwrap();
        let mut p = Parser::new(toks);
        let prog = p.parse_program().unwrap();
        emit_program(&prog, "test.df")
    }

    #[test]
    fn test_aug_assign_emit() {
        let cpp = compile("func main():\n    x = 0\n    x += 1\n");
        assert!(cpp.contains("x += 1"));
    }

    #[test]
    fn test_break_emit() {
        let cpp = compile("func main():\n    while True:\n        break\n");
        assert!(cpp.contains("break;"));
    }

    #[test]
    fn test_for_range_2arg() {
        let cpp = compile("func main():\n    for i in range(1, 10):\n        pass\n");
        assert!(cpp.contains("i < (10LL)"));
        assert!(cpp.contains("i = (1LL)"));
    }

    #[test]
    fn test_for_iter_emit() {
        let cpp = compile("func main():\n    for x in arr:\n        pass\n");
        assert!(cpp.contains("for (auto x : arr)"));
    }

    #[test]
    fn test_bitwise_emit() {
        let cpp = compile("func main():\n    x = a & b\n");
        assert!(cpp.contains("(a&b)"));
    }

    #[test]
    fn test_power_emit() {
        let cpp = compile("func main():\n    x = 2 ** 10\n");
        assert!(cpp.contains("deific::pow_int(2LL,10LL)"));
    }

    #[test]
    fn test_min_max() {
        let cpp = compile("func main():\n    x = min(a, b)\n    y = max(a, b)\n");
        assert!(cpp.contains("deific::deific_min"));
        assert!(cpp.contains("deific::deific_max"));
    }

    #[test]
    fn test_in_emit() {
        let cpp = compile("func main():\n    x = a in b\n");
        assert!(cpp.contains("deific::deific_in(a,b)"));
    }

    #[test]
    fn test_ref_param_emit() {
        let cpp = compile("func fill(arr: ref list[int]) -> void:\n    pass\n");
        assert!(cpp.contains("std::vector<long long>&arr"));
    }
}
