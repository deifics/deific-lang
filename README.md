<h1 align="center">Deific</h1>

<p align="center">
  <strong>Python's syntax. C++ speed.</strong>
</p>

<p align="center">
  <img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-blue.svg">
  <img alt="Built with Rust" src="https://img.shields.io/badge/built%20with-Rust-orange.svg">
  <img alt="Target: C++17" src="https://img.shields.io/badge/target-C%2B%2B17-00599C.svg">
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-lightgrey.svg">
  <img alt="CI" src="https://github.com/deifics/deific-lang/actions/workflows/ci.yml/badge.svg">
</p>

---

Deific is a fast, statically-compiled language with Python-like syntax. Write expressive, readable code — get a native binary that runs at full C++ speed. There's no runtime, no VM, no garbage collector. Just your code, transpiled to C++17, compiled by g++.

```python
struct Point:
    x: int
    y: int

func distance(a: Point, b: Point) -> float:
    dx = a.x - b.x
    dy = a.y - b.y
    return (dx * dx + dy * dy) ** 0.5

func main():
    p1 = Point(0, 0)
    p2 = Point(3, 4)
    print(distance(p1, p2))   # 5
```

```
$ deific run main.df
5
```

---

## Design principles

**Speed wins.** When ergonomics and performance conflict, Deific picks performance. Three deliberate breaks from Python:

| Area | Python | Deific |
|------|--------|--------|
| **Numbers** | Arbitrary precision | `int` = 64-bit `long long`; `bigint` is opt-in |
| **Assignment** | Reference + GC | Value-copy semantics; `ref` for in-place mutation |
| **Strings** | Unicode codepoints | Byte strings (`std::string`) |

**No magic.** Type inference uses C++ `auto` — only annotate empty containers and function signatures. The generated C++ is readable and can be inspected with `deific emit`.

**Zero dependencies.** The compiler is a single Rust binary. The emitted `.cpp` is self-contained.

---

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) (stable; GNU toolchain on Windows)
- `g++` with C++17 support — [MSYS2 ucrt64](https://www.msys2.org/) recommended on Windows

### Build from source

```powershell
# Windows (PowerShell) — must use GNU toolchain, not MSVC
cargo +stable-x86_64-pc-windows-gnu build --release
```

```bash
# Linux / macOS
cargo build --release
```

Binary at `target/release/deific` (or `deific.exe` on Windows).

### Pre-built releases

Download a pre-built binary from the [Releases](https://github.com/deifics/deific-lang/releases) page. You still need `g++` installed separately — Deific calls it to compile the generated C++.

---

## CLI

```
deific run   <file.df>              build to a temp binary and run immediately
deific build <file.df> [-o out]     compile to a named binary
deific build <file.df> --static     statically link libgcc/libstdc++ (portable binary)
deific emit  <file.df>              print generated C++ to stdout
deific test  <file.df>              run all test_* functions and report results
```

---

## Language reference

### Hello World

```python
func main():
    print("Hello, world!")
```

### Variables and type inference

```python
func main():
    x = 42          # int (long long)
    f = 3.14        # float (double)
    s = "hello"     # str (std::string)
    flag = True     # bool

    nums: list[int] = []       # annotate empty containers
    d: dict[str, int] = {}
```

### Types

| Deific | C++ | Notes |
|--------|-----|-------|
| `int` | `long long` | 64-bit signed |
| `float` | `double` | IEEE 754 |
| `str` | `std::string` | Byte string |
| `bool` | `bool` | `True` / `False` |
| `bigint` | `__int128` | GCC only, opt-in |
| `list[T]` | `std::vector<T>` | |
| `dict[K, V]` | `std::unordered_map<K, V>` | |
| `set[T]` | `std::unordered_set<T>` | |
| `tuple[A, B]` | `std::tuple<A, B>` | |

### Structs

```python
struct Point:
    x: int
    y: int

func main():
    p = Point(3, 4)   # positional construction
    print(p.x, p.y)   # field access
```

### Functions

```python
func add(a: int, b: int) -> int:
    return a + b

# Multiple return values
func minmax(nums: list[int]) -> (int, int):
    return min(nums), max(nums)

func main():
    lo, hi = minmax([3, 1, 4, 1, 5])
    print(lo, hi)
```

#### Generic functions

```python
func identity[T](x: T) -> T:
    return x
```

#### Reference parameters (in-place mutation)

```python
func fill(arr: ref list[int], val: int):
    for i in range(len(arr)):
        arr[i] = val
```

### Control flow

```python
if x > 0:
    print("positive")
elif x == 0:
    print("zero")
else:
    print("negative")

while x > 0:
    x -= 1

for i in range(10):
    if i == 5: break
    if i % 2 == 0: continue
    print(i)
```

### Global variables

```python
count = 0

func increment():
    global count
    count += 1

func main():
    increment()
    increment()
    print(count)   # 2
```

### Error handling

```python
func divide(a: int, b: int) -> int:
    if b == 0:
        panic("division by zero")
    return a // b

func main():
    assert(divide(10, 2) == 5)
    print("ok")
```

### Defer

```python
func main():
    defer print("runs last")
    print("runs first")
    # output: runs first \n runs last
```

`defer` runs at the end of the enclosing scope, in reverse order of declaration.

### Collections

```python
# List
nums = [1, 2, 3]
nums.append(4)
nums.sort()
squares = [x * x for x in nums]

# Dict
d: dict[str, int] = {"a": 1, "b": 2}
d["c"] = 3
exists = "a" in d

# Set
s: set[int] = {1, 2, 3}
s.add(4)
```

### Operators

```python
# Arithmetic
x = 10 + 3
x = 10 // 3   # floor division
x = 2 ** 8    # power

# Bitwise
a = x & y
a = x | y
a = x ^ y
a = x << 2
a = x >> 2

# Augmented
x += 5; x -= 5; x *= 2; x //= 2; x **= 2
x &= 0xFF; x |= 1; x ^= 0x10
```

### Built-in functions

| Function | Description |
|----------|-------------|
| `print(...)` | Print space-separated values |
| `input()` | Read a line from stdin |
| `int(x)` | Convert to 64-bit integer |
| `float(x)` | Convert to double |
| `str(x)` | Convert to string |
| `len(x)` | Length |
| `abs(x)` | Absolute value |
| `min(a, b)` / `min(lst)` | Minimum |
| `max(a, b)` / `max(lst)` | Maximum |
| `sum(lst)` | Sum |
| `sorted(lst)` | Sorted copy |
| `reversed(lst)` | Reversed copy |
| `enumerate(lst)` | `(index, value)` pairs |
| `zip(a, b)` | Element-wise pairs |
| `chr(n)` / `ord(s)` | Character conversion |
| `gcd(a, b)` / `lcm(a, b)` | Math |
| `pow_mod(base, exp, mod)` | Modular exponentiation |
| `read_int()` / `read_ints(n)` | Fast buffered integer input |
| `read_float()` / `read_floats(n)` | Fast buffered float input |
| `panic(msg)` | Print error and exit |
| `assert(cond)` | Panic if condition is false |

### Testing

```python
func add(a: int, b: int) -> int:
    return a + b

func test_add():
    assert(add(2, 3) == 5)
    assert(add(-1, 1) == 0)
```

```
$ deific test myfile.df
PASS test_add
1 passed, 0 failed
```

Any function named `test_*` with no parameters is a test. Use `deific test` to run them all.

---

## How it works

```
.df source → lexer → tokens → parser → AST → emit → .cpp → g++ -O2 → binary
```

| File | Role |
|------|------|
| `src/lexer.rs` | Tokeniser; emits `INDENT`/`DEDENT` for Python-style blocks |
| `src/parser.rs` | Recursive-descent + Pratt expression parser |
| `src/ast.rs` | AST node definitions |
| `src/emit.rs` | C++ code generator; inlines the buffered I/O runtime |
| `src/main.rs` | CLI entry point |

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT — see [LICENSE](LICENSE).
