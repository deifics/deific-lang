//! Deific compiler CLI.
//!
//! Subcommands:
//!   deific emit  <file.df>            print generated C++ to stdout (submittable)
//!   deific build <file.df> [-o out]   compile to a native binary via g++
//!   deific run   <file.df>            build to a temp binary and run it

mod ast;
mod emit;
mod lexer;
mod parser;

use std::path::{Path, PathBuf};
use std::process::{exit, Command};

fn main() {
    let args: Vec<String> = std::env::args().collect();
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

    let cpp = compile_to_cpp(&src, &src_path);

    match cmd {
        "emit" => {
            print!("{}", cpp);
        }
        "build" => {
            let out = parse_out_flag(&args).unwrap_or_else(|| default_bin(&src_path));
            build_cpp(&cpp, &out);
            eprintln!("deific: wrote {}", out.display());
        }
        "run" => {
            let tmp = std::env::temp_dir().join(format!("deific_{}", std::process::id()));
            let bin = with_exe_ext(&tmp);
            build_cpp(&cpp, &bin);
            let status = Command::new(&bin)
                .status()
                .unwrap_or_else(|e| fail(&format!("failed to run binary: {}", e)));
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

fn compile_to_cpp(src: &str, path: &Path) -> String {
    let toks = match lexer::lex(src) {
        Ok(t) => t,
        Err(e) => fail(&format!("{}:{}: lex error: {}", path.display(), e.line, e.msg)),
    };
    let mut p = parser::Parser::new(toks);
    let program = match p.parse_program() {
        Ok(pr) => pr,
        Err(e) => fail(&format!(
            "{}:{}: parse error: {}",
            path.display(),
            e.line,
            e.msg
        )),
    };
    if !program.funcs.iter().any(|f| f.name == "main") {
        fail("no `main` function found");
    }
    emit::emit_program(&program)
}

fn build_cpp(cpp: &str, out: &Path) {
    let cpp_path = out.with_extension("cpp");
    if let Err(e) = std::fs::write(&cpp_path, cpp) {
        fail(&format!("cannot write {}: {}", cpp_path.display(), e));
    }
    let status = Command::new("g++")
        .arg("-O2")
        .arg("-std=c++17")
        .arg("-o")
        .arg(out)
        .arg(&cpp_path)
        .status()
        .unwrap_or_else(|e| fail(&format!("failed to invoke g++: {}", e)));
    if !status.success() {
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
    eprintln!("usage: deific <emit|build|run> <file.df> [-o out]");
}

fn fail(msg: &str) -> ! {
    eprintln!("deific: {}", msg);
    exit(1);
}
