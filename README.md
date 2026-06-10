# Deific

**Python's syntax, transpiled to C++. C++ performance for competitive programming.**

Deific looks like Python but compiles to a native binary through C++. When Python's
ergonomics and C++'s speed conflict, **speed wins, every time.** That single rule
explains every design decision below.

```python
def main():
    n = int(input())
    a = read_ints(n)
    a.sort()
    total = 0
    for i in range(n):
        total = total + a[i]
    print(total)
    print(a)
```

```
$ deific run examples/sort.df
```

## The three deliberate breaks from Python

Each is a place where "true Python behavior" and "C++ performance" are physically
in conflict. Deific always picks performance:

| Area | Python | Deific | Why |
|------|--------|--------|-----|
| **Numbers** | arbitrary precision | `int` = 64-bit (`long long`); `bigint` is opt-in | bignum is a function call per op; CP thinks in `long long` |
| **Assignment** | reference + GC | value semantics (copy); `ref` for in-place | a GC is the thing that makes Python slow |
| **Strings** | Unicode codepoints | byte strings (`std::string`) | O(1) Unicode indexing hides real cost; CP is ASCII |

Internalize those three and the rest feels like Python.

## How it works

```
.df source ──lexer──> tokens ──parser──> AST ──emit──> .cpp ──g++──> native binary
```

- **Local type inference** leans on C++ `auto` — you only annotate function
  signatures (and empty containers).
- The emitted `.cpp` inlines a **buffered fast-I/O runtime**, so the generated
  file is self-contained and directly submittable to a judge.
- Integer literals carry an `LL` suffix so `int` is 64-bit everywhere.

## CLI

```
deific emit  <file.df>            # print generated C++ to stdout (submittable)
deific build <file.df> [-o out]   # compile to a native binary via g++
deific run   <file.df>            # build to a temp binary and run it
```

Requires a `g++` on PATH (C++17).

## Status — v0 vertical slice

This is the proof-of-thesis slice. Supported today: `def`/`return`, `for ... in
range(...)`, `while`, `if/elif/else`, arithmetic & comparison & boolean operators,
indexing, list `.sort()`/`.append()`/`.pop()`/`.reverse()`, and the built-ins
`print`, `input`, `int`, `len`, `read_int`, `read_ints`.

Not yet: comprehensions, dict/set, tuples, `bigint`, the `ref` annotation,
generics-as-templates, `#line` error mapping, and the bundled CP algorithms
library. See the design notes for the full roadmap.

## Building the compiler

```
cargo build --release
```

Pure-`std` Rust, zero dependencies (builds offline).
