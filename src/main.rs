//! Deific compiler CLI.
//!
//! Subcommands:
//!   deific emit  <file.df>              print generated C++ to stdout (submittable)
//!   deific build <file.df> [-o out]     compile to a native binary via g++
//!   deific build <file.df> --static     link statically (portable binary)
//!   deific run   <file.df>              build to a temp binary and run it
//!   deific test  <file.df>              run all test_* functions and report results

mod ast;
mod emit;
mod lexer;
mod parser;

use ast::Program as _; // ensure ast types are in scope for resolve_imports

use std::path::{Path, PathBuf};
use std::process::{exit, Command};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        usage();
        exit(2);
    }
    if args[1] == "--version" || args[1] == "-v" {
        println!("deific {}", env!("CARGO_PKG_VERSION"));
        exit(0);
    }
    if args.len() < 3 {
        usage();
        exit(2);
    }
    let cmd = args[1].as_str();
    let src_path = PathBuf::from(&args[2]);

    let src = match std::fs::read_to_string(&src_path) {
        Ok(s) => s,
        Err(e) => fail(&format!("cannot read {}: {}", src_path.display(), e)),
    };

    let is_static = args.iter().any(|a| a == "--static");

    match cmd {
        "emit" => {
            let cpp = compile_to_cpp(&src, &src_path, false);
            print!("{}", cpp);
        }
        "build" => {
            let cpp = compile_to_cpp(&src, &src_path, false);
            let out = parse_out_flag(&args).unwrap_or_else(|| default_bin(&src_path));
            build_cpp(&cpp, &out, is_static);
            eprintln!("deific: wrote {}", out.display());
        }
        "run" => {
            let cpp = compile_to_cpp(&src, &src_path, false);
            let tmp = std::env::temp_dir().join(format!("deific_{}", std::process::id()));
            let bin = with_exe_ext(&tmp);
            build_cpp(&cpp, &bin, false);
            let status = Command::new(&bin)
                .status()
                .unwrap_or_else(|e| fail(&format!("failed to run binary: {}", e)));
            let _ = std::fs::remove_file(&bin);
            exit(status.code().unwrap_or(1));
        }
        "test" => {
            let cpp = compile_to_cpp(&src, &src_path, true);
            let tmp = std::env::temp_dir().join(format!("deific_test_{}", std::process::id()));
            let bin = with_exe_ext(&tmp);
            build_cpp(&cpp, &bin, false);
            let status = Command::new(&bin)
                .status()
                .unwrap_or_else(|e| fail(&format!("failed to run test binary: {}", e)));
            let _ = std::fs::remove_file(&bin);
            exit(status.code().unwrap_or(1));
        }
        other => {
            eprintln!("deific: unknown command '{}'", other);
            usage();
            exit(2);
        }
    }
}

fn resolve_imports(program: &mut ast::Program, base_dir: &Path, seen: &mut std::collections::HashSet<std::path::PathBuf>) {
    let import_paths: Vec<String> = program.imports.drain(..).collect();
    for imp in import_paths {
        let candidate = base_dir.join(format!("{}.df", imp));
        let abs = match candidate.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                eprintln!("deific: warning: could not find module '{}'", imp);
                continue;
            }
        };
        if !seen.insert(abs.clone()) { continue; } // already loaded
        let src = match std::fs::read_to_string(&abs) {
            Ok(s) => s,
            Err(e) => fail(&format!("cannot read module '{}': {}", abs.display(), e)),
        };
        let toks = match lexer::lex(&src) {
            Ok(t) => t,
            Err(e) => fail(&format!("{}:{}: lex error: {}", abs.display(), e.line, e.msg)),
        };
        let mut p = parser::Parser::new(toks);
        let mut imported = match p.parse_program() {
            Ok(pr) => pr,
            Err(e) => fail(&format!("{}:{}: parse error: {}", abs.display(), e.line, e.msg)),
        };
        // Recursively resolve imports in the imported file
        let module_dir = abs.parent().unwrap_or(base_dir);
        resolve_imports(&mut imported, module_dir, seen);
        // Merge: imported definitions come first so they're available to the main file
        imported.structs.append(&mut program.structs);
        program.structs = imported.structs;
        imported.globals.append(&mut program.globals);
        program.globals = imported.globals;
        // Skip any `main` function from imported modules
        let fns: Vec<ast::Func> = imported.funcs.into_iter()
            .filter(|f| f.name != "main")
            .collect();
        let mut merged = fns;
        merged.append(&mut program.funcs);
        program.funcs = merged;
    }
}

fn compile_to_cpp(src: &str, path: &Path, test_mode: bool) -> String {
    let toks = match lexer::lex(src) {
        Ok(t) => t,
        Err(e) => fail(&format!("{}:{}: lex error: {}", path.display(), e.line, e.msg)),
    };
    let mut p = parser::Parser::new(toks);
    let mut program = match p.parse_program() {
        Ok(pr) => pr,
        Err(e) => fail(&format!("{}:{}: parse error: {}", path.display(), e.line, e.msg)),
    };
    let base_dir = path.parent().unwrap_or(Path::new("."));
    let mut seen = std::collections::HashSet::new();
    if let Ok(abs) = path.canonicalize() { seen.insert(abs); }
    resolve_imports(&mut program, base_dir, &mut seen);
    if test_mode {
        let (cpp, fns) = emit::emit_test_program(&program, &path.to_string_lossy());
        if fns.is_empty() {
            eprintln!("deific: warning: no test_* functions found");
        } else {
            eprintln!("deific: running {} test(s)", fns.len());
        }
        cpp
    } else {
        if !program.funcs.iter().any(|f| f.name == "main") {
            fail("no `main` function found");
        }
        emit::emit_program(&program, &path.to_string_lossy())
    }
}

fn find_gpp() -> &'static str {
    const CANDIDATES: &[&str] = &[
        "g++",
        r"C:\msys64\ucrt64\bin\g++.exe",
        r"C:\msys64\mingw64\bin\g++.exe",
    ];
    for &c in CANDIDATES {
        if Command::new(c).arg("--version").output().is_ok() {
            return c;
        }
    }
    "g++"
}

fn build_cpp(cpp: &str, out: &Path, static_link: bool) {
    let cpp_path = out.with_extension("cpp");
    if let Err(e) = std::fs::write(&cpp_path, cpp) {
        fail(&format!("cannot write {}: {}", cpp_path.display(), e));
    }
    let mut cmd = Command::new(find_gpp());
    cmd.arg("-O2").arg("-std=c++17").arg("-o").arg(out).arg(&cpp_path);
    if static_link {
        cmd.arg("-static-libgcc").arg("-static-libstdc++");
    }
    let result = cmd.output()
        .unwrap_or_else(|e| fail(&format!("failed to invoke g++: {}", e)));
    if !result.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&result.stderr));
    }
    if !result.status.success() {
        fail("g++ failed to compile the generated C++");
    }
}

fn parse_out_flag(args: &[String]) -> Option<PathBuf> {
    let i = args.iter().position(|a| a == "-o")?;
    args.get(i + 1).map(PathBuf::from)
}

fn default_bin(src: &Path) -> PathBuf {
    with_exe_ext(&src.with_extension(""))
}

fn with_exe_ext(p: &Path) -> PathBuf {
    if cfg!(windows) {
        p.with_extension("exe")
    } else {
        p.to_path_buf()
    }
}

fn usage() {
    eprintln!("usage: deific <emit|build|run|test> <file.df> [-o out] [--static]");
}

fn fail(msg: &str) -> ! {
    eprintln!("deific: {}", msg);
    exit(1);
}
