# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

Deific is a transpiler: Python-syntax `.df` source → C++17 → native binary. Built for competitive programming. North star: **when Python ergonomics and C++ speed conflict, speed wins.**

Three deliberate breaks from Python:
- `int` = 64-bit `long long` (not arbitrary precision); literals get `LL` suffix
- Assignment is value-copy semantics — no GC; `ref` annotation planned for in-place mutation
- Strings are byte strings (`std::string`), not Unicode codepoints

## Build & Run

**Always use PowerShell, not the Bash tool**, for anything that invokes `cargo` or `g++`. The Bash tool's sandbox silently blocks `cc1plus`/`as` sub-processes — `g++` exits 1 with no diagnostic. `g++` is MSYS2 ucrt64 at `C:\msys64\ucrt64\bin`.

```powershell
# Build the compiler (must use GNU toolchain, not MSVC)
cargo +stable-x86_64-pc-windows-gnu build --release

# Try an example
.\target\release\deific.exe run examples\sort.df

# Emit generated C++ to stdout
.\target\release\deific.exe emit examples\sort.df

# Build a .df file to a named binary
.\target\release\deific.exe build examples\sort.df -o sort.exe
```

**Feeding stdin to compiled programs**: Don't pipe via PowerShell `|` — it re-encodes to UTF-16 and corrupts `read_int`/`read_ints`. Use a file redirect instead:

```powershell
cmd /c ".\sort.exe < in.txt"
```

## Testing

No automated test suite yet. Manual testing workflow:

```powershell
# Compile and immediately run an example
.\target\release\deific.exe run examples\sort.df

# Inspect generated C++ before running it
.\target\release\deific.exe emit examples\sort.df
```

## Architecture

Pipeline: `.df` → **Lexer** → tokens → **Parser** → AST → **Emit** → `.cpp` → `g++ -O2 -std=c++17` → binary

All five modules are in `src/`:

| File | Role |
|------|------|
| `main.rs` | CLI (`emit` / `build` / `run`), orchestrates the full pipeline, error reporting |
| `lexer.rs` | Line-based tokenizer; emits INDENT/DEDENT for Python-style blocks; tabs normalized to 4 spaces |
| `parser.rs` | Recursive-descent + Pratt expression parsing; produces the AST |
| `ast.rs` | AST node definitions — `Program`, `Func`, statements, expressions, `Type` enum |
| `emit.rs` | Walks the AST and generates C++; inlines a buffered fast-I/O runtime (`FastIn`/`FastOut`) |

### Key emit details
- Local variables use `auto` (C++ infers types); function signatures use explicit Deific types mapped to C++ equivalents
- First assignment in a scope → `auto name = …`; subsequent → `name = …` (scope membership tracked in `emit.rs`)
- `list` → `std::vector`; `.append()` → `.push_back()`, `.sort()` → `std::sort`, `.pop()` → `.pop_back()`, `.reverse()` → `std::reverse`
- The emitted `.cpp` is self-contained and submittable directly to online judges

### Adding a new language feature
The typical change touches three files in order:
1. `lexer.rs` — add any new token/keyword
2. `parser.rs` — parse new syntax into an AST node
3. `ast.rs` — add the new node variant
4. `emit.rs` — emit C++ for the new node

### Zero external dependencies
`Cargo.toml` has no `[dependencies]`. Keep it that way unless there is a compelling reason.
