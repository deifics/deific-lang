# Contributing to Deific

## Building

Requires Rust (stable GNU toolchain) and MSYS2 `g++` on Windows, or any `g++` with C++17 support on Linux/macOS.

```powershell
# Windows (PowerShell) — always use PowerShell, not Bash, for cargo/g++ invocations
cargo +stable-x86_64-pc-windows-gnu build --release

# Linux / macOS
cargo build --release
```

## Running examples

```powershell
.\target\release\deific.exe run examples\sort.df
.\target\release\deific.exe emit examples\sort.df   # inspect generated C++
```

## Running tests

```powershell
cargo test
.\target\release\deific.exe test examples\hello_world.df  # language-level test runner
```

## Project layout

| Path | Purpose |
|------|---------|
| `src/lexer.rs` | Tokeniser — INDENT/DEDENT, keywords |
| `src/parser.rs` | Recursive-descent + Pratt expression parser |
| `src/ast.rs` | AST node definitions |
| `src/emit.rs` | C++ code generator; inlines the Deific runtime |
| `src/main.rs` | CLI (`emit` / `build` / `run` / `test`) |
| `examples/` | Sample `.df` programs |

## Adding a language feature

Touch files in this order:
1. `lexer.rs` — new token or keyword
2. `parser.rs` — parse into an AST node
3. `ast.rs` — add the node variant
4. `emit.rs` — generate C++ for it

## Pull requests

- One logical change per PR
- Include or update an example in `examples/` that exercises the new feature
- `cargo test` must pass
